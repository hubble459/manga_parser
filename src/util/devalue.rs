//! Decoder for the [devalue](https://github.com/Rich-Harris/devalue) format
//! (`application/json+devalue`), which frameworks like Astro and SvelteKit
//! use to embed page state in a `<script>` tag.
//!
//! The payload is a flat JSON array: element 0 is the root value, and inside
//! objects/arrays every integer is a reference to another element of that
//! array. Special forms are tagged arrays (`["Map", k, v, ...]`,
//! `["Date", "..."]`, ...) and negative integers are constants (undefined,
//! NaN, infinities, -0, array holes).
//!
//! The result is a plain `serde_json::Value`, so values JSON can't represent
//! are mapped onto the closest JSON equivalent: `Map` becomes an object (keys
//! stringified), `Set` an array, `Date`/`URL`/`BigInt`/`RegExp` strings, and
//! every special constant `null`.

use serde_json::{Map, Value};

use crate::error::ScrapeError;

/// Deep enough for any real page state, shallow enough to never overflow the
/// stack on a malicious payload.
const MAX_DEPTH: usize = 256;

pub fn parse(text: &str) -> Result<Value, ScrapeError> {
    let values: Vec<Value> = serde_json::from_str(text)
        .map_err(|e| ScrapeError::WebScrapingError(format!("Invalid devalue payload: {e}")))?;
    if values.is_empty() {
        return Ok(Value::Null);
    }
    let mut hydrated: Vec<Option<Value>> = vec![None; values.len()];
    let mut in_progress = vec![false; values.len()];
    Ok(Hydrator {
        values: &values,
        hydrated: &mut hydrated,
        in_progress: &mut in_progress,
    }
    .hydrate(0, 0))
}

struct Hydrator<'a> {
    values: &'a [Value],
    hydrated: &'a mut [Option<Value>],
    in_progress: &'a mut [bool],
}

impl Hydrator<'_> {
    fn hydrate_ref(&mut self, reference: &Value, depth: usize) -> Value {
        match reference.as_i64() {
            Some(index) if index >= 0 => self.hydrate(index as usize, depth),
            // -1 undefined, -2 hole, -3 NaN, -4 Infinity, -5 -Infinity, -6 -0
            Some(-6) => Value::from(0),
            _ => Value::Null,
        }
    }

    fn hydrate(&mut self, index: usize, depth: usize) -> Value {
        if index >= self.values.len() || depth > MAX_DEPTH {
            return Value::Null;
        }
        if let Some(value) = &self.hydrated[index] {
            return value.clone();
        }
        // A cycle can't be represented in JSON; cut it.
        if self.in_progress[index] {
            return Value::Null;
        }
        self.in_progress[index] = true;
        let values = self.values;
        let value = self.hydrate_value(&values[index], depth);
        self.in_progress[index] = false;
        self.hydrated[index] = Some(value.clone());
        value
    }

    fn hydrate_value(&mut self, value: &Value, depth: usize) -> Value {
        match value {
            Value::Array(items) => match items.first() {
                Some(Value::String(tag)) => self.hydrate_tagged(tag, &items[1..], depth),
                _ => Value::Array(items.iter().map(|item| self.hydrate_ref(item, depth + 1)).collect()),
            },
            Value::Object(fields) => Value::Object(
                fields
                    .iter()
                    .map(|(key, item)| (key.clone(), self.hydrate_ref(item, depth + 1)))
                    .collect(),
            ),
            primitive => primitive.clone(),
        }
    }

    fn hydrate_tagged(&mut self, tag: &str, args: &[Value], depth: usize) -> Value {
        match tag {
            "Map" | "null" => {
                let mut object = Map::new();
                for pair in args.chunks(2) {
                    let key = self.hydrate_ref(&pair[0], depth + 1);
                    let value = pair.get(1).map_or(Value::Null, |v| self.hydrate_ref(v, depth + 1));
                    object.insert(key_to_string(key), value);
                }
                Value::Object(object)
            }
            "Set" => Value::Array(args.iter().map(|item| self.hydrate_ref(item, depth + 1)).collect()),
            "Object" => args.first().map_or(Value::Null, |item| self.hydrate_ref(item, depth + 1)),
            "Date" | "URL" | "URLSearchParams" | "BigInt" => match args.first() {
                Some(Value::String(s)) => Value::String(s.clone()),
                Some(reference) => self.hydrate_ref(reference, depth + 1),
                None => Value::Null,
            },
            "RegExp" => {
                let source = args.first().and_then(Value::as_str).unwrap_or_default();
                let flags = args.get(1).and_then(Value::as_str).unwrap_or_default();
                Value::String(format!("/{source}/{flags}"))
            }
            // A type this decoder doesn't support.
            _ => Value::Null,
        }
    }
}

fn key_to_string(key: Value) -> String {
    match key {
        Value::String(s) => s,
        other => other.to_string(),
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::parse;

    #[test]
    fn test_references_and_objects() {
        let value = parse(r#"[{"id":1,"tags":2},"abc",[3,4],"a","b"]"#).unwrap();
        assert_eq!(value, json!({"id": "abc", "tags": ["a", "b"]}));
    }

    #[test]
    fn test_map_set_date_url() {
        let value = parse(
            r#"[["Map",1,2,3,4],"chapters",["Set",5,6],"url",["URL",7],1,2,"https://example.com/a"]"#,
        )
        .unwrap();
        assert_eq!(value, json!({"chapters": [1, 2], "url": "https://example.com/a"}));

        let value = parse(r#"[{"at":1},["Date","2026-06-03T19:48:08.795Z"]]"#).unwrap();
        assert_eq!(value, json!({"at": "2026-06-03T19:48:08.795Z"}));
    }

    #[test]
    fn test_special_constants() {
        let value = parse(r#"[[-1,-3,-4,-6,1],5]"#).unwrap();
        assert_eq!(value, json!([null, null, null, 0, 5]));
    }

    #[test]
    fn test_shared_and_cyclic_references() {
        // Shared references are duplicated, cycles are cut instead of recursing forever.
        let value = parse(r#"[{"a":1,"b":1},{"self":0}]"#).unwrap();
        assert_eq!(value, json!({"a": {"self": null}, "b": {"self": null}}));
    }

    #[test]
    fn test_invalid_payload() {
        assert!(parse("not json").is_err());
        assert_eq!(parse("[]").unwrap(), serde_json::Value::Null);
    }
}
