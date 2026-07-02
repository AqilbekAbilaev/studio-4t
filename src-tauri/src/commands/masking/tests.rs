use super::{apply_rules, MaskRule};
use mongodb::bson::{doc, Bson};

fn rule(field: &str, strategy: &str) -> MaskRule {
    MaskRule {
        field: field.to_string(),
        strategy: strategy.to_string(),
        keep_start: None,
        keep_end: None,
        mask_char: None,
        replacement: None,
    }
}

#[test]
fn redact_replaces_with_token() {
    let mut d = doc! { "name": "Alice", "keep": "me" };
    apply_rules(&mut d, &[rule("name", "redact")]).unwrap();
    assert_eq!(d.get_str("name").unwrap(), "***");
    assert_eq!(d.get_str("keep").unwrap(), "me");
}

#[test]
fn redact_custom_replacement() {
    let mut d = doc! { "name": "Alice" };
    let mut r = rule("name", "redact");
    r.replacement = Some("[hidden]".to_string());
    apply_rules(&mut d, &[r]).unwrap();
    assert_eq!(d.get_str("name").unwrap(), "[hidden]");
}

#[test]
fn hash_is_deterministic_and_hides_value() {
    let mut a = doc! { "email": "a@x.com" };
    let mut b = doc! { "email": "a@x.com" };
    let mut c = doc! { "email": "different@x.com" };
    apply_rules(&mut a, &[rule("email", "hash")]).unwrap();
    apply_rules(&mut b, &[rule("email", "hash")]).unwrap();
    apply_rules(&mut c, &[rule("email", "hash")]).unwrap();
    let ha = a.get_str("email").unwrap();
    let hb = b.get_str("email").unwrap();
    let hc = c.get_str("email").unwrap();
    assert_ne!(ha, "a@x.com");            // original hidden
    assert_eq!(ha, hb);                    // same input → same hash (joins survive)
    assert_ne!(ha, hc);                    // different input → different hash
    assert_eq!(ha.len(), 16);              // 16 hex chars
}

#[test]
fn partial_keeps_start_and_end() {
    let mut d = doc! { "card": "4111111111111234" };
    let mut r = rule("card", "partial");
    r.keep_start = Some(0);
    r.keep_end = Some(4);
    apply_rules(&mut d, &[r]).unwrap();
    assert_eq!(d.get_str("card").unwrap(), "************1234");
}

#[test]
fn partial_keeps_both_ends() {
    let mut d = doc! { "email": "john@example.com" };
    let mut r = rule("email", "partial");
    r.keep_start = Some(2);
    r.keep_end = Some(4);
    apply_rules(&mut d, &[r]).unwrap();
    // "jo" + masked middle + ".com"
    assert_eq!(d.get_str("email").unwrap(), "jo**********.com");
}

#[test]
fn partial_short_string_fully_masked() {
    let mut d = doc! { "pin": "12" };
    let mut r = rule("pin", "partial");
    r.keep_start = Some(2);
    r.keep_end = Some(2);
    apply_rules(&mut d, &[r]).unwrap();
    assert_eq!(d.get_str("pin").unwrap(), "**");
}

#[test]
fn partial_custom_mask_char() {
    let mut d = doc! { "x": "abcdef" };
    let mut r = rule("x", "partial");
    r.keep_start = Some(1);
    r.keep_end = Some(1);
    r.mask_char = Some("#".to_string());
    apply_rules(&mut d, &[r]).unwrap();
    assert_eq!(d.get_str("x").unwrap(), "a####f");
}

#[test]
fn nullify_sets_null() {
    let mut d = doc! { "ssn": "123-45-6789" };
    apply_rules(&mut d, &[rule("ssn", "nullify")]).unwrap();
    assert_eq!(d.get("ssn"), Some(&Bson::Null));
}

#[test]
fn remove_drops_field() {
    let mut d = doc! { "secret": "x", "keep": 1_i32 };
    apply_rules(&mut d, &[rule("secret", "remove")]).unwrap();
    assert!(!d.contains_key("secret"));
    assert!(d.contains_key("keep"));
}

#[test]
fn nested_dotted_path() {
    let mut d = doc! { "contact": { "email": "a@x.com", "phone": "555" } };
    apply_rules(&mut d, &[rule("contact.email", "redact")]).unwrap();
    let contact = d.get_document("contact").unwrap();
    assert_eq!(contact.get_str("email").unwrap(), "***");
    assert_eq!(contact.get_str("phone").unwrap(), "555");
}

#[test]
fn absent_field_is_a_noop() {
    let mut d = doc! { "a": 1_i32 };
    apply_rules(&mut d, &[rule("missing", "redact"), rule("also.missing", "remove")]).unwrap();
    assert_eq!(d.get_i32("a").unwrap(), 1);
    assert_eq!(d.len(), 1);
}

#[test]
fn hash_works_on_non_string_values() {
    let mut d = doc! { "n": 42_i64 };
    apply_rules(&mut d, &[rule("n", "hash")]).unwrap();
    // Coerced to its text form, then hashed to a 16-hex string.
    assert_eq!(d.get_str("n").unwrap().len(), 16);
}

#[test]
fn unknown_strategy_errors() {
    let mut d = doc! { "a": 1_i32 };
    assert!(apply_rules(&mut d, &[rule("a", "bogus")]).is_err());
}

#[test]
fn multiple_rules_apply_in_order() {
    let mut d = doc! { "name": "Alice", "email": "a@x.com", "age": 30_i32 };
    apply_rules(
        &mut d,
        &[rule("name", "redact"), rule("email", "hash"), rule("age", "remove")],
    )
    .unwrap();
    assert_eq!(d.get_str("name").unwrap(), "***");
    assert_eq!(d.get_str("email").unwrap().len(), 16);
    assert!(!d.contains_key("age"));
}
