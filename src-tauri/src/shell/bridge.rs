// The MongoDB `db` bridge for the embedded shell.
//
// A single native function `__mongo({ collection, method, args })` is registered
// on each JS context. A JS preamble builds `db` as a Proxy whose property access
// (`db.users`) yields a collection object whose methods forward to `__mongo`.
//
// `boa`'s native functions are synchronous, so each driver call is run to
// completion with `Handle::block_on`. The current connection (client + database
// + runtime handle) lives in a shared slot that the worker rebinds before every
// evaluation — see engine.rs.

use std::cell::RefCell;
use std::rc::Rc;

use boa_engine::{js_string, Context, JsError, JsValue, NativeFunction, Source};
use boa_gc::{Finalize, Trace};
use boa_engine::error::JsNativeError;
use mongodb::bson;
use mongodb::{Client, Collection};
use tokio::runtime::Handle;

/// The live connection a shell session is bound to. Rebound before each eval.
pub(super) struct DbInner {
    pub client: Client,
    pub db_name: String,
    pub handle: Handle,
}

/// Capture handed to the native `__mongo` function. Holds only an `Rc` to the
/// connection slot — no GC-traceable members, hence the empty trace.
#[derive(Trace, Finalize)]
#[boa_gc(unsafe_empty_trace)]
pub(super) struct DbContext {
    pub slot: Rc<RefCell<Option<DbInner>>>,
}

/// Register `__mongo` and install the `db` Proxy preamble on a context.
pub(super) fn install_db(context: &mut Context, slot: Rc<RefCell<Option<DbInner>>>) {
    let captures = DbContext { slot: slot };
    let mongo = NativeFunction::from_copy_closure_with_captures(
        |_this, args, captures: &DbContext, context| mongo_call(args, captures, context),
        captures,
    );
    let _ = context.register_global_callable(js_string!("__mongo"), 1, mongo);
    let _ = context.eval(Source::from_bytes(DB_PREAMBLE));
}

/// `db` is a Proxy so `db.<anyCollection>` resolves dynamically (including
/// not-yet-created collections, which MongoDB creates on first write).
const DB_PREAMBLE: &str = r#"
    (function () {
        function makeCollection(name) {
            function call(method, args) {
                return globalThis.__mongo({ collection: name, method: method, args: args });
            }
            return {
                find:            function (q, p) { return call('find', [q || {}, p || {}]); },
                findOne:         function (q, p) { return call('findOne', [q || {}, p || {}]); },
                insertOne:       function (d)    { return call('insertOne', [d]); },
                insertMany:      function (d)    { return call('insertMany', [d]); },
                updateOne:       function (q, u) { return call('updateOne', [q, u]); },
                updateMany:      function (q, u) { return call('updateMany', [q, u]); },
                replaceOne:      function (q, r) { return call('replaceOne', [q, r]); },
                deleteOne:       function (q)    { return call('deleteOne', [q]); },
                deleteMany:      function (q)    { return call('deleteMany', [q]); },
                countDocuments:  function (q)    { return call('countDocuments', [q || {}]); },
                aggregate:       function (p)    { return call('aggregate', [p || []]); },
            };
        }
        var base = {
            getCollection: function (name) { return makeCollection(name); },
            runCommand:    function (cmd)  { return globalThis.__mongo({ collection: null, method: 'runCommand', args: [cmd] }); },
        };
        globalThis.db = new Proxy(base, {
            get: function (target, prop) {
                if (prop in target) return target[prop];
                if (typeof prop === 'symbol') return undefined;
                return makeCollection(prop);
            }
        });
    })();
"#;

/// The native dispatcher: decode the operation, run it on the driver, hand the
/// result back to JS. A failed operation throws a JS error so it surfaces in the
/// transcript as a normal exception.
fn mongo_call(args: &[JsValue], captures: &DbContext, context: &mut Context) -> JsResult {
    let op = match args.first() {
        Some(value) => value,
        None => return Err(throw("__mongo: missing operation descriptor")),
    };
    let op_json = match op.to_json(context) {
        Ok(Some(value)) => value,
        Ok(None) => return Err(throw("__mongo: operation is undefined")),
        Err(e) => return Err(e),
    };

    // Copy out the live connection so we don't hold the RefCell borrow across
    // the blocking driver call.
    let bound = {
        let slot = captures.slot.borrow();
        match slot.as_ref() {
            Some(inner) => (inner.client.clone(), inner.db_name.clone(), inner.handle.clone()),
            None => return Err(throw("no database is bound to this shell session")),
        }
    };
    let (client, db_name, handle) = bound;

    match run_op(&client, &db_name, &handle, &op_json) {
        Ok(value) => JsValue::from_json(&value, context),
        Err(message) => Err(throw(&message)),
    }
}

type JsResult = boa_engine::JsResult<JsValue>;

fn throw(message: &str) -> JsError {
    JsNativeError::error().with_message(message.to_string()).into()
}

/// Dispatch one decoded `{ collection, method, args }` operation to the driver,
/// blocking on the async call via the provided runtime handle.
fn run_op(
    client: &Client,
    db_name: &str,
    handle: &Handle,
    op: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let method = match op.get("method").and_then(|value| value.as_str()) {
        Some(value) => value,
        None => return Err(String::from("operation has no method")),
    };
    let empty: Vec<serde_json::Value> = Vec::new();
    let args = match op.get("args").and_then(|value| value.as_array()) {
        Some(value) => value,
        None => &empty,
    };
    let database = client.database(db_name);

    handle.block_on(async {
        if method == "runCommand" {
            let command = match arg_doc(args, 0) {
                Ok(doc) => doc,
                Err(e) => return Err(e),
            };
            return match database.run_command(command).await {
                Ok(doc) => Ok(bson_doc_to_json(doc)),
                Err(e) => Err(e.to_string()),
            };
        }

        let collection_name = match op.get("collection").and_then(|value| value.as_str()) {
            Some(value) => value,
            None => return Err(String::from("operation has no collection")),
        };
        let collection = database.collection::<bson::Document>(collection_name);

        match method {
            "find" => exec_find(&collection, args).await,
            "findOne" => exec_find_one(&collection, args).await,
            "insertOne" => exec_insert_one(&collection, args).await,
            "insertMany" => exec_insert_many(&collection, args).await,
            "updateOne" => exec_update(&collection, args, false).await,
            "updateMany" => exec_update(&collection, args, true).await,
            "replaceOne" => exec_replace_one(&collection, args).await,
            "deleteOne" => exec_delete(&collection, args, false).await,
            "deleteMany" => exec_delete(&collection, args, true).await,
            "countDocuments" => exec_count(&collection, args).await,
            "aggregate" => exec_aggregate(&collection, args).await,
            other => Err(format!("unsupported shell method: {}", other)),
        }
    })
}

// ── argument / result conversion ──────────────────────────────────────────

/// Decode a JS-object argument into a BSON document. Uses bson's serde Extended
/// JSON decoding — the same mechanism as `commands::parse_ejson_document`, so
/// `ObjectId("…")` / `{ $oid }` and friends round-trip correctly.
fn to_document(value: &serde_json::Value) -> Result<bson::Document, String> {
    match serde_json::from_value::<bson::Bson>(value.clone()) {
        Ok(bson::Bson::Document(doc)) => Ok(doc),
        Ok(bson::Bson::Null) => Ok(bson::Document::new()),
        Ok(_) => Err(String::from("expected a document argument")),
        Err(e) => Err(e.to_string()),
    }
}

/// Document argument at `index`, defaulting to an empty document when absent.
fn arg_doc(args: &[serde_json::Value], index: usize) -> Result<bson::Document, String> {
    match args.get(index) {
        Some(value) => to_document(value),
        None => Ok(bson::Document::new()),
    }
}

/// BSON → EJSON-preserving JSON (same conversion the find/aggregate commands use).
fn bson_doc_to_json(doc: bson::Document) -> serde_json::Value {
    serde_json::Value::from(bson::Bson::Document(doc))
}

// ── per-method executors ──────────────────────────────────────────────────

async fn exec_find(
    collection: &Collection<bson::Document>,
    args: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    let filter = match arg_doc(args, 0) {
        Ok(doc) => doc,
        Err(e) => return Err(e),
    };
    let mut query = collection.find(filter);
    if args.len() > 1 {
        let projection = match arg_doc(args, 1) {
            Ok(doc) => doc,
            Err(e) => return Err(e),
        };
        if !projection.is_empty() {
            query = query.projection(projection);
        }
    }
    let mut cursor = match query.await {
        Ok(value) => value,
        Err(e) => return Err(e.to_string()),
    };
    let mut docs = Vec::new();
    loop {
        let has_next = match cursor.advance().await {
            Ok(value) => value,
            Err(e) => return Err(e.to_string()),
        };
        if !has_next {
            break;
        }
        let doc: bson::Document = match cursor.deserialize_current() {
            Ok(value) => value,
            Err(e) => return Err(e.to_string()),
        };
        docs.push(bson_doc_to_json(doc));
    }
    Ok(serde_json::Value::Array(docs))
}

async fn exec_find_one(
    collection: &Collection<bson::Document>,
    args: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    let filter = match arg_doc(args, 0) {
        Ok(doc) => doc,
        Err(e) => return Err(e),
    };
    let mut query = collection.find_one(filter);
    if args.len() > 1 {
        let projection = match arg_doc(args, 1) {
            Ok(doc) => doc,
            Err(e) => return Err(e),
        };
        if !projection.is_empty() {
            query = query.projection(projection);
        }
    }
    match query.await {
        Ok(Some(doc)) => Ok(bson_doc_to_json(doc)),
        Ok(None) => Ok(serde_json::Value::Null),
        Err(e) => Err(e.to_string()),
    }
}

async fn exec_insert_one(
    collection: &Collection<bson::Document>,
    args: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    let doc = match arg_doc(args, 0) {
        Ok(value) => value,
        Err(e) => return Err(e),
    };
    let result = match collection.insert_one(doc).await {
        Ok(value) => value,
        Err(e) => return Err(e.to_string()),
    };
    let mut out = serde_json::Map::new();
    out.insert(String::from("acknowledged"), serde_json::Value::Bool(true));
    out.insert(
        String::from("insertedId"),
        serde_json::Value::from(result.inserted_id),
    );
    Ok(serde_json::Value::Object(out))
}

async fn exec_insert_many(
    collection: &Collection<bson::Document>,
    args: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    let array = match args.first().and_then(|value| value.as_array()) {
        Some(value) => value,
        None => return Err(String::from("insertMany expects an array of documents")),
    };
    let mut docs = Vec::new();
    for item in array {
        match to_document(item) {
            Ok(doc) => docs.push(doc),
            Err(e) => return Err(e),
        }
    }
    let result = match collection.insert_many(docs).await {
        Ok(value) => value,
        Err(e) => return Err(e.to_string()),
    };
    let mut ids = serde_json::Map::new();
    for (index, id) in result.inserted_ids {
        ids.insert(index.to_string(), serde_json::Value::from(id));
    }
    let mut out = serde_json::Map::new();
    out.insert(String::from("acknowledged"), serde_json::Value::Bool(true));
    out.insert(
        String::from("insertedCount"),
        serde_json::Value::from(ids.len()),
    );
    out.insert(String::from("insertedIds"), serde_json::Value::Object(ids));
    Ok(serde_json::Value::Object(out))
}

async fn exec_update(
    collection: &Collection<bson::Document>,
    args: &[serde_json::Value],
    many: bool,
) -> Result<serde_json::Value, String> {
    let filter = match arg_doc(args, 0) {
        Ok(value) => value,
        Err(e) => return Err(e),
    };
    let update = match arg_doc(args, 1) {
        Ok(value) => value,
        Err(e) => return Err(e),
    };
    let result = if many {
        collection.update_many(filter, update).await
    } else {
        collection.update_one(filter, update).await
    };
    match result {
        Ok(value) => Ok(update_result_to_json(value)),
        Err(e) => Err(e.to_string()),
    }
}

async fn exec_replace_one(
    collection: &Collection<bson::Document>,
    args: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    let filter = match arg_doc(args, 0) {
        Ok(value) => value,
        Err(e) => return Err(e),
    };
    let replacement = match arg_doc(args, 1) {
        Ok(value) => value,
        Err(e) => return Err(e),
    };
    match collection.replace_one(filter, replacement).await {
        Ok(value) => Ok(update_result_to_json(value)),
        Err(e) => Err(e.to_string()),
    }
}

async fn exec_delete(
    collection: &Collection<bson::Document>,
    args: &[serde_json::Value],
    many: bool,
) -> Result<serde_json::Value, String> {
    let filter = match arg_doc(args, 0) {
        Ok(value) => value,
        Err(e) => return Err(e),
    };
    let result = if many {
        collection.delete_many(filter).await
    } else {
        collection.delete_one(filter).await
    };
    match result {
        Ok(value) => {
            let mut out = serde_json::Map::new();
            out.insert(String::from("acknowledged"), serde_json::Value::Bool(true));
            out.insert(
                String::from("deletedCount"),
                serde_json::Value::from(value.deleted_count),
            );
            Ok(serde_json::Value::Object(out))
        }
        Err(e) => Err(e.to_string()),
    }
}

async fn exec_count(
    collection: &Collection<bson::Document>,
    args: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    let filter = match arg_doc(args, 0) {
        Ok(value) => value,
        Err(e) => return Err(e),
    };
    match collection.count_documents(filter).await {
        Ok(value) => Ok(serde_json::Value::from(value)),
        Err(e) => Err(e.to_string()),
    }
}

async fn exec_aggregate(
    collection: &Collection<bson::Document>,
    args: &[serde_json::Value],
) -> Result<serde_json::Value, String> {
    let array = match args.first().and_then(|value| value.as_array()) {
        Some(value) => value,
        None => return Err(String::from("aggregate expects a pipeline array")),
    };
    let mut stages = Vec::new();
    for item in array {
        match to_document(item) {
            Ok(doc) => stages.push(doc),
            Err(e) => return Err(e),
        }
    }
    let mut cursor = match collection.aggregate(stages).await {
        Ok(value) => value,
        Err(e) => return Err(e.to_string()),
    };
    let mut docs = Vec::new();
    loop {
        let has_next = match cursor.advance().await {
            Ok(value) => value,
            Err(e) => return Err(e.to_string()),
        };
        if !has_next {
            break;
        }
        let doc: bson::Document = match cursor.deserialize_current() {
            Ok(value) => value,
            Err(e) => return Err(e.to_string()),
        };
        docs.push(bson_doc_to_json(doc));
    }
    Ok(serde_json::Value::Array(docs))
}

fn update_result_to_json(result: mongodb::results::UpdateResult) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    out.insert(String::from("acknowledged"), serde_json::Value::Bool(true));
    out.insert(
        String::from("matchedCount"),
        serde_json::Value::from(result.matched_count),
    );
    out.insert(
        String::from("modifiedCount"),
        serde_json::Value::from(result.modified_count),
    );
    match result.upserted_id {
        Some(id) => {
            out.insert(String::from("upsertedId"), serde_json::Value::from(id));
        }
        None => {}
    }
    serde_json::Value::Object(out)
}
