use crate::error::AppError;
use mongodb::bson;
use serde::{Deserialize, Serialize};
use tauri::State;

use super::{collect_values, next_document, MAX_QUERY_TIME, AppContext};

// One reschema transform, chosen by the user in the modal. Internally tagged on
// `op` so the frontend sends `{ "op": "rename", "from": …, "to": … }`. All field
// names are dotted paths (e.g. "contact.email"), so nested moves are just a rename
// between two dotted paths.
#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum ReschemaOp {
    // Copy `from` into `to`, then drop `from`.
    Rename { from: String, to: String },
    // Drop `field`.
    Remove { field: String },
    // Convert `field` to `to_type` ($convert target: string/int/long/double/
    // decimal/bool/date/objectId). A value that can't be converted is left as-is
    // (onError/onNull keep the original) so one bad document never aborts the run.
    ChangeType {
        field: String,
        #[serde(rename = "toType")]
        to_type: String,
    },
    // Move/rename a nested field — identical to Rename, spelled separately so the
    // UI can label it, since both operands are dotted paths.
    Move { from: String, to: String },
}

// Where an apply writes its result: rewrite the source collection in place
// (`$merge`) or emit a brand-new collection (`$out`).
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ReschemaTarget {
    // "in_place" | "new_collection"
    pub mode: String,
    #[serde(default)]
    pub new_name: Option<String>,
}

// Preview payload: the first N documents as they are now (`before`) and as the
// pipeline would rewrite them (`after`), so the modal can show them side by side.
#[derive(Serialize)]
pub struct ReschemaPreview {
    pub before: Vec<serde_json::Value>,
    pub after: Vec<serde_json::Value>,
}

// A `$field` path reference, the aggregation-expression spelling of "the current
// value at this path".
fn field_ref(path: &str) -> String {
    format!("${}", path)
}

// Turn one rename/move (`from` → `to`) into its two stages: set the destination to
// the source's value, then unset the source. Shared by the Rename and Move arms.
fn rename_stages(from: &str, to: &str) -> [bson::Document; 2] {
    let mut set_doc = bson::Document::new();
    set_doc.insert(to.to_string(), field_ref(from));
    [
        bson::doc! { "$set": set_doc },
        bson::doc! { "$unset": from.to_string() },
    ]
}

/// Build the aggregation pipeline for an ordered list of ops. Pure and DB-free so
/// it can be unit-tested; both preview and apply run whatever this produces (apply
/// appends a `$merge`/`$out` write stage of its own).
pub fn build_pipeline(ops: &[ReschemaOp]) -> Vec<bson::Document> {
    let mut stages: Vec<bson::Document> = Vec::new();
    for op in ops {
        match op {
            ReschemaOp::Rename { from, to }
            | ReschemaOp::Move { from, to } => {
                let pair = rename_stages(from, to);
                stages.push(pair[0].clone());
                stages.push(pair[1].clone());
            }
            ReschemaOp::Remove { field } => {
                stages.push(bson::doc! { "$unset": field.to_string() });
            }
            ReschemaOp::ChangeType { field, to_type } => {
                // onError/onNull fall back to the original value, so a value that
                // can't be converted is left untouched instead of failing the op.
                let convert = bson::doc! {
                    "$convert": {
                        "input": field_ref(field),
                        "to": to_type.to_string(),
                        "onError": field_ref(field),
                        "onNull": field_ref(field),
                    }
                };
                let mut set_doc = bson::Document::new();
                set_doc.insert(field.to_string(), convert);
                stages.push(bson::doc! { "$set": set_doc });
            }
        }
    }
    stages
}

/// Preview the reschema: the first `limit` documents as-is, and the same window run
/// through the transform pipeline. The `$limit` is placed *before* the transforms so
/// the "before" and "after" samples cover the same documents and the preview never
/// scans the whole collection.
#[tauri::command]
pub async fn reschema_preview(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    ops: Vec<ReschemaOp>,
    limit: i64,
) -> Result<ReschemaPreview, AppError> {
    let col = match ctx.collection(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let sample = if limit <= 0 { 20 } else { limit };

    // "Before": the first N documents in natural order.
    let mut before_cursor = match col.find(bson::doc! {}).limit(sample).max_time(MAX_QUERY_TIME).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let before = match collect_values(&mut before_cursor).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    // "After": the same N documents, transformed. $limit first keeps it to the same
    // window and off the whole collection.
    let mut stages: Vec<bson::Document> = vec![bson::doc! { "$limit": sample }];
    stages.extend(build_pipeline(&ops));
    let mut after_cursor = match col.aggregate(stages).max_time(MAX_QUERY_TIME).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let after = match collect_values(&mut after_cursor).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    Ok(ReschemaPreview { before: before, after: after })
}

/// Apply the reschema across the whole collection. In-place mode rewrites the source
/// via `$merge` (replace on `_id`); new-collection mode writes a fresh collection via
/// `$out`. Returns the number of documents in the resulting collection.
#[tauri::command]
pub async fn reschema_apply(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    ops: Vec<ReschemaOp>,
    target: ReschemaTarget,
) -> Result<usize, AppError> {
    let col = match ctx.collection(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    let mut stages = build_pipeline(&ops);

    // The write stage differs by target; `counted` is whichever collection we count
    // afterwards to report how many documents were written.
    let counted = match target.mode.as_str() {
        "new_collection" => {
            let new_name = match target.new_name.as_deref().map(str::trim) {
                Some(name) if !name.is_empty() => name.to_string(),
                _ => {
                    return Err(AppError::Validation(
                        "Enter a name for the new collection".to_string(),
                    ))
                }
            };
            if new_name == collection {
                return Err(AppError::Validation(
                    "New collection name must differ from the source".to_string(),
                ));
            }
            stages.push(bson::doc! { "$out": new_name.clone() });
            match ctx.collection(&id, &database, &new_name).await {
                Ok(val) => val,
                Err(e) => return Err(e),
            }
        }
        "in_place" => {
            stages.push(bson::doc! {
                "$merge": {
                    "into": collection.clone(),
                    "whenMatched": "replace",
                    "whenNotMatched": "insert",
                }
            });
            col.clone()
        }
        other => {
            return Err(AppError::Validation(format!(
                "Unknown apply mode: {other}"
            )))
        }
    };

    // $out / $merge yield no documents, but the write only completes once the cursor
    // is driven to exhaustion, so drain it before counting.
    let mut cursor = match col.aggregate(stages).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    loop {
        match next_document(&mut cursor).await {
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(e) => return Err(e),
        }
    }

    let count = match counted.count_documents(bson::doc! {}).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(count as usize)
}

#[cfg(test)]
mod tests {
    use super::{build_pipeline, ReschemaOp};
    use mongodb::bson::doc;

    #[test]
    fn rename_sets_then_unsets() {
        let ops = vec![ReschemaOp::Rename {
            from: "a".to_string(),
            to: "b".to_string(),
        }];
        let pipeline = build_pipeline(&ops);
        assert_eq!(
            pipeline,
            vec![doc! { "$set": { "b": "$a" } }, doc! { "$unset": "a" }]
        );
    }

    #[test]
    fn move_uses_dotted_paths_like_rename() {
        let ops = vec![ReschemaOp::Move {
            from: "a.b".to_string(),
            to: "c.d".to_string(),
        }];
        let pipeline = build_pipeline(&ops);
        assert_eq!(
            pipeline,
            vec![doc! { "$set": { "c.d": "$a.b" } }, doc! { "$unset": "a.b" }]
        );
    }

    #[test]
    fn remove_unsets_the_field() {
        let ops = vec![ReschemaOp::Remove {
            field: "legacy".to_string(),
        }];
        let pipeline = build_pipeline(&ops);
        assert_eq!(pipeline, vec![doc! { "$unset": "legacy" }]);
    }

    #[test]
    fn change_type_converts_and_keeps_original_on_error() {
        let ops = vec![ReschemaOp::ChangeType {
            field: "age".to_string(),
            to_type: "int".to_string(),
        }];
        let pipeline = build_pipeline(&ops);
        assert_eq!(
            pipeline,
            vec![doc! {
                "$set": {
                    "age": {
                        "$convert": {
                            "input": "$age",
                            "to": "int",
                            "onError": "$age",
                            "onNull": "$age",
                        }
                    }
                }
            }]
        );
    }

    #[test]
    fn change_type_supports_dotted_paths() {
        let ops = vec![ReschemaOp::ChangeType {
            field: "meta.count".to_string(),
            to_type: "long".to_string(),
        }];
        let pipeline = build_pipeline(&ops);
        assert_eq!(
            pipeline,
            vec![doc! {
                "$set": {
                    "meta.count": {
                        "$convert": {
                            "input": "$meta.count",
                            "to": "long",
                            "onError": "$meta.count",
                            "onNull": "$meta.count",
                        }
                    }
                }
            }]
        );
    }

    #[test]
    fn ops_apply_in_order() {
        let ops = vec![
            ReschemaOp::Rename {
                from: "a".to_string(),
                to: "b".to_string(),
            },
            ReschemaOp::Remove {
                field: "c".to_string(),
            },
        ];
        let pipeline = build_pipeline(&ops);
        assert_eq!(
            pipeline,
            vec![
                doc! { "$set": { "b": "$a" } },
                doc! { "$unset": "a" },
                doc! { "$unset": "c" },
            ]
        );
    }
}
