use crate::error::AppError;
use serde::Serialize;

// Translate a simple SQL SELECT into the pieces of a MongoDB find query, the way
// Studio-3T's "SQL Query" surface does. This is deliberately pure (no I/O) so it
// is fully unit-testable and works with no live connection. Scope is single-table
// SELECT with WHERE / ORDER BY / LIMIT / OFFSET; aggregate functions and GROUP BY
// are intentionally out of scope for now and rejected with a clear message.

#[derive(Serialize, Debug)]
pub struct MqlQuery {
    pub collection: String,
    // Pretty-printed JSON strings so the UI can drop them straight into the query
    // bar (filter / projection / sort) or render the equivalent shell command.
    pub filter: String,
    pub projection: String,
    pub sort: String,
    pub limit: Option<i64>,
    pub skip: Option<i64>,
}

// ── Ordered JSON value ─────────────────────────────────────────────
// serde_json::Value orders object keys alphabetically without the preserve_order
// feature, which would corrupt multi-key sort documents (ORDER BY b, a). This
// small ordered emitter keeps insertion order so output is faithful and tests are
// deterministic.
enum J {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

fn json_string(value: &str) -> String {
    match serde_json::to_string(value) {
        Ok(val) => val,
        Err(_) => format!("\"{value}\""),
    }
}

impl J {
    fn to_pretty(&self, level: usize) -> String {
        let pad = "  ".repeat(level);
        let pad1 = "  ".repeat(level + 1);
        match self {
            J::Null => "null".to_string(),
            J::Bool(value) => value.to_string(),
            J::Int(value) => value.to_string(),
            J::Float(value) => match serde_json::Number::from_f64(*value) {
                Some(num) => num.to_string(),
                None => "null".to_string(),
            },
            J::Str(value) => json_string(value),
            J::Arr(items) => {
                if items.is_empty() {
                    return "[]".to_string();
                }
                let inner: Vec<String> = items
                    .iter()
                    .map(|item| format!("{pad1}{}", item.to_pretty(level + 1)))
                    .collect();
                format!("[\n{}\n{pad}]", inner.join(",\n"))
            }
            J::Obj(entries) => {
                if entries.is_empty() {
                    return "{}".to_string();
                }
                let inner: Vec<String> = entries
                    .iter()
                    .map(|(key, value)| {
                        format!("{pad1}{}: {}", json_string(key), value.to_pretty(level + 1))
                    })
                    .collect();
                format!("{{\n{}\n{pad}}}", inner.join(",\n"))
            }
        }
    }
}

// ── Tokenizer ──────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq)]
enum Tok {
    // A bare word: keyword or identifier, decided by the parser (case-insensitive).
    Word(String),
    Str(String),
    Int(i64),
    Float(f64),
    Sym(String), // = != <> < <= > >= ( ) , * .
    Eof,
}

fn tokenize(sql: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = sql.chars().collect();
    let mut tokens: Vec<Tok> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // String literal: single or double quoted, with doubled-quote escaping.
        if c == '\'' || c == '"' {
            let quote = c;
            i += 1;
            let mut value = String::new();
            let mut closed = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch == quote {
                    if i + 1 < chars.len() && chars[i + 1] == quote {
                        value.push(quote);
                        i += 2;
                        continue;
                    }
                    closed = true;
                    i += 1;
                    break;
                }
                value.push(ch);
                i += 1;
            }
            if !closed {
                return Err("Unterminated string literal".to_string());
            }
            tokens.push(Tok::Str(value));
            continue;
        }
        // Number: digits with optional single decimal point. A leading sign is
        // handled by the value parser as unary minus so "a-1" still tokenizes sanely.
        if c.is_ascii_digit() {
            let start = i;
            let mut seen_dot = false;
            while i < chars.len() {
                let ch = chars[i];
                if ch.is_ascii_digit() {
                    i += 1;
                } else if ch == '.' && !seen_dot {
                    seen_dot = true;
                    i += 1;
                } else {
                    break;
                }
            }
            let text: String = chars[start..i].iter().collect();
            if seen_dot {
                match text.parse::<f64>() {
                    Ok(val) => tokens.push(Tok::Float(val)),
                    Err(_) => return Err(format!("Invalid number: {text}")),
                }
            } else {
                match text.parse::<i64>() {
                    Ok(val) => tokens.push(Tok::Int(val)),
                    Err(_) => return Err(format!("Invalid number: {text}")),
                }
            }
            continue;
        }
        // Identifier / keyword: letters, digits, underscore, and dots for field paths.
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() {
                let ch = chars[i];
                if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                    i += 1;
                } else {
                    break;
                }
            }
            let text: String = chars[start..i].iter().collect();
            tokens.push(Tok::Word(text));
            continue;
        }
        // Multi-char and single-char operators.
        let two: String = if i + 1 < chars.len() {
            chars[i..i + 2].iter().collect()
        } else {
            String::new()
        };
        if two == "!=" || two == "<>" || two == "<=" || two == ">=" {
            tokens.push(Tok::Sym(two));
            i += 2;
            continue;
        }
        match c {
            '=' | '<' | '>' | '(' | ')' | ',' | '*' => {
                tokens.push(Tok::Sym(c.to_string()));
                i += 1;
            }
            ';' => {
                // Statement terminator; ignore so a trailing ';' is accepted.
                i += 1;
            }
            '-' => {
                tokens.push(Tok::Sym("-".to_string()));
                i += 1;
            }
            other => return Err(format!("Unexpected character: {other}")),
        }
    }
    tokens.push(Tok::Eof);
    Ok(tokens)
}

// ── Expression tree ────────────────────────────────────────────────
enum Cond {
    Eq(J),
    Ne(J),
    Lt(J),
    Lte(J),
    Gt(J),
    Gte(J),
    In(Vec<J>),
    Nin(Vec<J>),
    Regex(String),
    NotRegex(String),
    Between(J, J),
    IsNull,
    IsNotNull,
}

enum Expr {
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Leaf(String, Cond),
}

struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        match self.tokens.get(self.pos) {
            Some(val) => val,
            None => &Tok::Eof,
        }
    }

    fn next(&mut self) -> Tok {
        let tok = match self.tokens.get(self.pos) {
            Some(val) => val.clone(),
            None => Tok::Eof,
        };
        self.pos += 1;
        tok
    }

    // Consume a word if it matches `kw` case-insensitively; return whether it did.
    fn eat_keyword(&mut self, kw: &str) -> bool {
        if let Tok::Word(word) = self.peek() {
            if word.eq_ignore_ascii_case(kw) {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn peek_keyword(&self, kw: &str) -> bool {
        if let Tok::Word(word) = self.peek() {
            return word.eq_ignore_ascii_case(kw);
        }
        false
    }

    fn eat_sym(&mut self, sym: &str) -> bool {
        if let Tok::Sym(value) = self.peek() {
            if value == sym {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    // A field name is a bare word that is not a reserved keyword.
    fn expect_field(&mut self) -> Result<String, String> {
        match self.next() {
            Tok::Word(word) => {
                if is_reserved(&word) {
                    return Err(format!("Expected a field name, found reserved word `{word}`"));
                }
                Ok(word)
            }
            other => Err(format!("Expected a field name, found {}", describe(&other))),
        }
    }
}

fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Word(word) => format!("`{word}`"),
        Tok::Str(_) => "a string".to_string(),
        Tok::Int(value) => format!("`{value}`"),
        Tok::Float(value) => format!("`{value}`"),
        Tok::Sym(value) => format!("`{value}`"),
        Tok::Eof => "end of input".to_string(),
    }
}

const RESERVED: &[&str] = &[
    "select", "from", "where", "order", "by", "limit", "offset", "skip", "and", "or", "like",
    "in", "is", "null", "not", "between", "asc", "desc", "as", "group",
];

fn is_reserved(word: &str) -> bool {
    RESERVED.iter().any(|kw| word.eq_ignore_ascii_case(kw))
}

// ── Value parsing ──────────────────────────────────────────────────
fn parse_value(parser: &mut Parser) -> Result<J, String> {
    let negate = parser.eat_sym("-");
    match parser.next() {
        Tok::Int(value) => {
            let v = if negate { -value } else { value };
            Ok(J::Int(v))
        }
        Tok::Float(value) => {
            let v = if negate { -value } else { value };
            Ok(J::Float(v))
        }
        Tok::Str(value) => {
            if negate {
                return Err("Cannot negate a string literal".to_string());
            }
            Ok(J::Str(value))
        }
        Tok::Word(word) => {
            if negate {
                return Err(format!("Cannot negate `{word}`"));
            }
            if word.eq_ignore_ascii_case("true") {
                Ok(J::Bool(true))
            } else if word.eq_ignore_ascii_case("false") {
                Ok(J::Bool(false))
            } else if word.eq_ignore_ascii_case("null") {
                Ok(J::Null)
            } else {
                Err(format!("Expected a value, found `{word}`"))
            }
        }
        other => Err(format!("Expected a value, found {}", describe(&other))),
    }
}

fn parse_value_list(parser: &mut Parser) -> Result<Vec<J>, String> {
    if !parser.eat_sym("(") {
        return Err("Expected `(` after IN".to_string());
    }
    let mut values: Vec<J> = Vec::new();
    if parser.eat_sym(")") {
        return Err("IN list cannot be empty".to_string());
    }
    loop {
        let value = match parse_value(parser) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        values.push(value);
        if parser.eat_sym(",") {
            continue;
        }
        if parser.eat_sym(")") {
            break;
        }
        return Err(format!("Expected `,` or `)` in IN list, found {}", describe(parser.peek())));
    }
    Ok(values)
}

// SQL LIKE → anchored regex, escaping regex metacharacters and mapping % → .* and _ → .
fn like_to_regex(pattern: &str) -> String {
    let mut out = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '%' => out.push_str(".*"),
            '_' => out.push('.'),
            '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    out.push('$');
    out
}

// ── Condition / expression parsing ─────────────────────────────────
fn parse_comparison(parser: &mut Parser) -> Result<Expr, String> {
    let field = match parser.expect_field() {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    // IS [NOT] NULL
    if parser.eat_keyword("is") {
        let is_not = parser.eat_keyword("not");
        if !parser.eat_keyword("null") {
            return Err("Expected NULL after IS".to_string());
        }
        let cond = if is_not { Cond::IsNotNull } else { Cond::IsNull };
        return Ok(Expr::Leaf(field, cond));
    }

    // NOT LIKE / NOT IN
    if parser.eat_keyword("not") {
        if parser.eat_keyword("like") {
            match parser.next() {
                Tok::Str(pattern) => return Ok(Expr::Leaf(field, Cond::NotRegex(like_to_regex(&pattern)))),
                other => return Err(format!("Expected a pattern string after NOT LIKE, found {}", describe(&other))),
            }
        }
        if parser.eat_keyword("in") {
            let values = match parse_value_list(parser) {
                Ok(val) => val,
                Err(e) => return Err(e),
            };
            return Ok(Expr::Leaf(field, Cond::Nin(values)));
        }
        return Err("Expected LIKE or IN after NOT".to_string());
    }

    // LIKE
    if parser.eat_keyword("like") {
        match parser.next() {
            Tok::Str(pattern) => return Ok(Expr::Leaf(field, Cond::Regex(like_to_regex(&pattern)))),
            other => return Err(format!("Expected a pattern string after LIKE, found {}", describe(&other))),
        }
    }

    // IN
    if parser.eat_keyword("in") {
        let values = match parse_value_list(parser) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        return Ok(Expr::Leaf(field, Cond::In(values)));
    }

    // BETWEEN a AND b
    if parser.eat_keyword("between") {
        let low = match parse_value(parser) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        if !parser.eat_keyword("and") {
            return Err("Expected AND in BETWEEN".to_string());
        }
        let high = match parse_value(parser) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        return Ok(Expr::Leaf(field, Cond::Between(low, high)));
    }

    // Comparison operators.
    let op = match parser.next() {
        Tok::Sym(value) => value,
        other => return Err(format!("Expected an operator after `{field}`, found {}", describe(&other))),
    };
    let value = match parse_value(parser) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let cond = match op.as_str() {
        "=" => Cond::Eq(value),
        "!=" | "<>" => Cond::Ne(value),
        "<" => Cond::Lt(value),
        "<=" => Cond::Lte(value),
        ">" => Cond::Gt(value),
        ">=" => Cond::Gte(value),
        other => return Err(format!("Unsupported operator `{other}`")),
    };
    Ok(Expr::Leaf(field, cond))
}

fn parse_primary(parser: &mut Parser) -> Result<Expr, String> {
    if parser.eat_sym("(") {
        let expr = match parse_or(parser) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        if !parser.eat_sym(")") {
            return Err("Expected `)`".to_string());
        }
        return Ok(expr);
    }
    parse_comparison(parser)
}

fn parse_and(parser: &mut Parser) -> Result<Expr, String> {
    let first = match parse_primary(parser) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    // No `AND` follows — a single term, returned as-is (no And wrapper).
    if !parser.eat_keyword("and") {
        return Ok(first);
    }
    let mut parts = vec![first];
    loop {
        let next = match parse_primary(parser) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        parts.push(next);
        if !parser.eat_keyword("and") {
            break;
        }
    }
    Ok(Expr::And(parts))
}

fn parse_or(parser: &mut Parser) -> Result<Expr, String> {
    let first = match parse_and(parser) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    // No `OR` follows — a single term, returned as-is (no Or wrapper).
    if !parser.eat_keyword("or") {
        return Ok(first);
    }
    let mut parts = vec![first];
    loop {
        let next = match parse_and(parser) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        parts.push(next);
        if !parser.eat_keyword("or") {
            break;
        }
    }
    Ok(Expr::Or(parts))
}

// ── Expr → JSON ────────────────────────────────────────────────────
fn cond_to_j(cond: Cond) -> J {
    match cond {
        Cond::Eq(value) => value,
        Cond::Ne(value) => J::Obj(vec![("$ne".to_string(), value)]),
        Cond::Lt(value) => J::Obj(vec![("$lt".to_string(), value)]),
        Cond::Lte(value) => J::Obj(vec![("$lte".to_string(), value)]),
        Cond::Gt(value) => J::Obj(vec![("$gt".to_string(), value)]),
        Cond::Gte(value) => J::Obj(vec![("$gte".to_string(), value)]),
        Cond::In(values) => J::Obj(vec![("$in".to_string(), J::Arr(values))]),
        Cond::Nin(values) => J::Obj(vec![("$nin".to_string(), J::Arr(values))]),
        Cond::Regex(pattern) => J::Obj(vec![("$regex".to_string(), J::Str(pattern))]),
        Cond::NotRegex(pattern) => J::Obj(vec![(
            "$not".to_string(),
            J::Obj(vec![("$regex".to_string(), J::Str(pattern))]),
        )]),
        Cond::Between(low, high) => {
            J::Obj(vec![("$gte".to_string(), low), ("$lte".to_string(), high)])
        }
        Cond::IsNull => J::Null,
        Cond::IsNotNull => J::Obj(vec![("$ne".to_string(), J::Null)]),
    }
}

// True when every part is a single-key object and all keys are distinct, so an
// AND can be flattened into one filter object instead of an $and array.
fn can_merge(parts: &[J]) -> bool {
    let mut keys: Vec<&str> = Vec::new();
    for part in parts {
        match part {
            J::Obj(entries) if entries.len() == 1 => {
                let key = entries[0].0.as_str();
                if keys.iter().any(|existing| *existing == key) {
                    return false;
                }
                keys.push(key);
            }
            _ => return false,
        }
    }
    true
}

fn expr_to_j(expr: Expr) -> J {
    match expr {
        Expr::Leaf(field, cond) => J::Obj(vec![(field, cond_to_j(cond))]),
        Expr::Or(children) => {
            let parts: Vec<J> = children.into_iter().map(expr_to_j).collect();
            J::Obj(vec![("$or".to_string(), J::Arr(parts))])
        }
        Expr::And(children) => {
            let parts: Vec<J> = children.into_iter().map(expr_to_j).collect();
            if can_merge(&parts) {
                let mut merged: Vec<(String, J)> = Vec::new();
                for part in parts {
                    if let J::Obj(entries) = part {
                        for entry in entries {
                            merged.push(entry);
                        }
                    }
                }
                J::Obj(merged)
            } else {
                J::Obj(vec![("$and".to_string(), J::Arr(parts))])
            }
        }
    }
}

// ── Top-level translate ────────────────────────────────────────────
pub(crate) fn sql_to_mql(sql: &str) -> Result<MqlQuery, String> {
    let tokens = match tokenize(sql) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let mut parser = Parser { tokens: tokens, pos: 0 };

    if !parser.eat_keyword("select") {
        return Err("Query must start with SELECT".to_string());
    }

    // Projection list.
    let mut projection: Vec<(String, J)> = Vec::new();
    if parser.eat_sym("*") {
        // All fields: empty projection.
    } else {
        loop {
            let field = match parser.expect_field() {
                Ok(val) => val,
                Err(e) => return Err(e),
            };
            // Reject function calls (COUNT(...), SUM(...)) — aggregates are out of scope.
            if parser.peek() == &Tok::Sym("(".to_string()) {
                return Err(format!(
                    "Aggregate/function `{field}(...)` is not supported yet — only plain column selection"
                ));
            }
            projection.push((field, J::Int(1)));
            // Optional `AS alias` — accepted and ignored (projection uses the source field).
            if parser.eat_keyword("as") {
                match parser.expect_field() {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
            if parser.eat_sym(",") {
                continue;
            }
            break;
        }
    }

    if !parser.eat_keyword("from") {
        return Err("Expected FROM".to_string());
    }
    let collection = match parser.expect_field() {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    // WHERE.
    let filter = if parser.eat_keyword("where") {
        let expr = match parse_or(&mut parser) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        expr_to_j(expr)
    } else {
        J::Obj(Vec::new())
    };

    // ORDER BY.
    let mut sort: Vec<(String, J)> = Vec::new();
    if parser.eat_keyword("order") {
        if !parser.eat_keyword("by") {
            return Err("Expected BY after ORDER".to_string());
        }
        loop {
            let field = match parser.expect_field() {
                Ok(val) => val,
                Err(e) => return Err(e),
            };
            let dir = if parser.eat_keyword("desc") {
                -1
            } else {
                parser.eat_keyword("asc");
                1
            };
            sort.push((field, J::Int(dir)));
            if parser.eat_sym(",") {
                continue;
            }
            break;
        }
    }

    // LIMIT / OFFSET / SKIP, in either order.
    let mut limit: Option<i64> = None;
    let mut skip: Option<i64> = None;
    loop {
        if parser.peek_keyword("limit") {
            parser.eat_keyword("limit");
            match parser.next() {
                Tok::Int(value) => limit = Some(value),
                other => return Err(format!("Expected a number after LIMIT, found {}", describe(&other))),
            }
            continue;
        }
        if parser.peek_keyword("offset") || parser.peek_keyword("skip") {
            parser.next();
            match parser.next() {
                Tok::Int(value) => skip = Some(value),
                other => return Err(format!("Expected a number after OFFSET, found {}", describe(&other))),
            }
            continue;
        }
        break;
    }

    if parser.peek() != &Tok::Eof {
        return Err(format!("Unexpected trailing input: {}", describe(parser.peek())));
    }

    Ok(MqlQuery {
        collection: collection,
        filter: filter.to_pretty(0),
        projection: J::Obj(projection).to_pretty(0),
        sort: J::Obj(sort).to_pretty(0),
        limit: limit,
        skip: skip,
    })
}

/// Translate a SQL SELECT statement into the parts of an equivalent MongoDB find
/// query. Pure and connection-free.
#[tauri::command]
pub fn translate_sql(sql: String) -> Result<MqlQuery, AppError> {
    match sql_to_mql(&sql) {
        Ok(val) => Ok(val),
        Err(message) => Err(AppError::Sql(message)),
    }
}

#[cfg(test)]
#[path = "sql.test.rs"]
mod tests;
