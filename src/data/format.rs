//! A JSON pretty-printer tuned for files people also edit by hand:
//! small objects (a leaf task, a log entry) stay on one line, anything that
//! holds a list of objects (a task with subtasks) always expands.

use serde_json::Value;

const WIDTH: usize = 150;

pub fn to_pretty(v: &Value) -> String {
    let mut out = String::new();
    write(v, 0, 0, &mut out);
    out.push('\n');
    out
}

/// `indent`: nesting depth in spaces; `col`: column where this value starts.
fn write(v: &Value, indent: usize, col: usize, out: &mut String) {
    let one_line = inline(v);
    let fits = col + one_line.chars().count() <= WIDTH;
    match v {
        Value::Object(m) if !m.is_empty() && (!fits || holds_object_list(v)) => {
            out.push_str("{\n");
            for (i, (k, val)) in m.iter().enumerate() {
                let key = format!("{}: ", Value::String(k.clone()));
                pad(out, indent + 2);
                out.push_str(&key);
                write(val, indent + 2, indent + 2 + key.chars().count(), out);
                out.push_str(if i + 1 < m.len() { ",\n" } else { "\n" });
            }
            pad(out, indent);
            out.push('}');
        }
        Value::Array(a) if !a.is_empty() && (!fits || a.iter().any(Value::is_object)) => {
            out.push_str("[\n");
            for (i, val) in a.iter().enumerate() {
                pad(out, indent + 2);
                write(val, indent + 2, indent + 2, out);
                out.push_str(if i + 1 < a.len() { ",\n" } else { "\n" });
            }
            pad(out, indent);
            out.push(']');
        }
        _ => out.push_str(&one_line),
    }
}

fn holds_object_list(v: &Value) -> bool {
    v.as_object()
        .is_some_and(|m| m.values().any(|x| x.as_array().is_some_and(|a| a.iter().any(Value::is_object))))
}

/// Single-line form with a space after `:` and `,`.
fn inline(v: &Value) -> String {
    match v {
        Value::Object(m) if m.is_empty() => "{}".into(),
        Value::Object(m) => {
            let parts: Vec<String> = m
                .iter()
                .map(|(k, x)| format!("{}: {}", Value::String(k.clone()), inline(x)))
                .collect();
            format!("{{ {} }}", parts.join(", "))
        }
        Value::Array(a) => format!("[{}]", a.iter().map(inline).collect::<Vec<_>>().join(", ")),
        other => other.to_string(),
    }
}

fn pad(out: &mut String, n: usize) {
    out.extend(std::iter::repeat_n(' ', n));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn leaves_stay_inline_and_parents_expand() {
        let v = json!({
            "name": "P",
            "tasks": [
                { "id": "A", "title": "a", "subtasks": [{ "id": "B", "title": "b" }] },
                { "id": "C", "title": "c", "tags": ["x", "y"] }
            ]
        });
        let s = to_pretty(&v);
        assert!(s.contains(r#"    { "id": "C", "title": "c", "tags": ["x", "y"] }"#), "{s}");
        assert!(s.contains(r#"      "subtasks": ["#), "{s}");
        assert!(s.contains(r#"        { "id": "B", "title": "b" }"#), "{s}");
        // Output is still valid JSON with the same content.
        assert_eq!(serde_json::from_str::<Value>(&s).unwrap(), v);
    }

    #[test]
    fn long_objects_wrap() {
        let v = json!({ "tasks": [{ "id": "A", "title": "x".repeat(160) }] });
        let s = to_pretty(&v);
        assert!(s.contains("\n      \"title\": "), "{s}");
    }
}
