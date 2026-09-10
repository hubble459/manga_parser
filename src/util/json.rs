use std::collections::HashMap;

use serde_json::Value;

use crate::error::ScrapeError;

/// Flatten a JSON array (found at `array_pointer`, an RFC 6901 JSON Pointer
/// such as "/data/chapters") into a synthetic HTML document: one
/// `<div class="json-item" data-<key>="<value>">` per array element, with
/// one attribute per scalar field. This lets the existing CSS
/// attribute-selector machinery (`text_selection: { type: attributes }`)
/// read JSON API responses without any changes to the selector engine.
pub fn flatten_json_array_to_html(
    json: &Value,
    array_pointer: &str,
    url_template: Option<&str>,
    host: &str,
    base_url: &str,
) -> Result<String, ScrapeError> {
    let array = json
        .pointer(array_pointer)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ScrapeError::WebScrapingError(format!(
                "JSON path '{array_pointer}' not found or not an array"
            ))
        })?;

    let mut html = String::from("<html><body>");
    for item in array {
        let Some(obj) = item.as_object() else {
            continue;
        };

        let mut attrs: HashMap<String, String> = HashMap::new();
        for (key, value) in obj {
            if let Some(s) = scalar_to_string(value) {
                attrs.insert(sanitize_key(key), s);
            }
        }

        if let Some(template) = url_template {
            let mut computed = template.replace("{host}", host).replace("{url}", base_url);
            for (key, value) in &attrs {
                computed = computed.replace(&format!("{{{key}}}"), value);
            }
            attrs.insert("__url".to_string(), computed);
        }

        html.push_str("<div class=\"json-item\"");
        for (key, value) in &attrs {
            html.push_str(&format!(" data-{key}=\"{}\"", escape_html_attr(value)));
        }
        html.push_str("></div>");
    }
    html.push_str("</body></html>");

    Ok(html)
}

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

fn sanitize_key(key: &str) -> String {
    key.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect()
}

fn escape_html_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::flatten_json_array_to_html;

    #[test]
    fn flattens_scalar_fields_into_attributes() {
        let json = json!({
            "data": {
                "chapters": [
                    {"chapter_num": 266, "chapter_name": "Chapter 266", "chapter_slug": "chapter-266"}
                ]
            }
        });

        let html = flatten_json_array_to_html(&json, "/data/chapters", None, "example.com", "https://example.com/manga/foo").unwrap();

        assert!(html.contains("data-chapter_num=\"266\""));
        assert!(html.contains("data-chapter_name=\"Chapter 266\""));
        assert!(html.contains("data-chapter_slug=\"chapter-266\""));
    }

    #[test]
    fn omits_null_fields() {
        let json = json!({"items": [{"a": "x", "b": null}]});

        let html = flatten_json_array_to_html(&json, "/items", None, "example.com", "https://example.com").unwrap();

        assert!(html.contains("data-a=\"x\""));
        assert!(!html.contains("data-b"));
    }

    #[test]
    fn json_stringifies_nested_values() {
        let json = json!({"items": [{"tags": ["a", "b"]}]});

        let html = flatten_json_array_to_html(&json, "/items", None, "example.com", "https://example.com").unwrap();

        assert!(html.contains("data-tags=\"[&quot;a&quot;,&quot;b&quot;]\""));
    }

    #[test]
    fn computes_url_template_from_outer_context_and_item_fields() {
        let json = json!({"items": [{"chapter_slug": "chapter-266"}]});

        let html = flatten_json_array_to_html(
            &json,
            "/items",
            Some("{url}/{chapter_slug}"),
            "example.com",
            "https://example.com/manga/foo",
        )
        .unwrap();

        assert!(html.contains("data-__url=\"https://example.com/manga/foo/chapter-266\""));
    }

    #[test]
    fn errors_when_pointer_is_missing_or_not_an_array() {
        let json = json!({"data": {"chapters": "not-an-array"}});

        let result = flatten_json_array_to_html(&json, "/data/chapters", None, "example.com", "https://example.com");

        assert!(result.is_err());
    }
}
