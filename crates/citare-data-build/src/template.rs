//! Template variable substitution, replicating Python's `string.Template` and
//! `reporters_db.utils.process_variables` / `recursive_substitute`.

use std::collections::HashMap;

/// Replace `$name` or `${name}` occurrences in `template` with values from
/// `vars`.  Unknown variables are left as-is (mirrors Python's
/// `string.Template.safe_substitute`).
pub fn safe_substitute(template: &str, vars: &HashMap<String, String>) -> String {
    let chars: Vec<char> = template.chars().collect();
    let mut result = String::with_capacity(template.len());
    let mut i = 0;

    while i < chars.len() {
        if chars[i] != '$' {
            result.push(chars[i]);
            i += 1;
            continue;
        }

        // We have a '$' — look ahead.
        let next = i + 1;
        if next >= chars.len() {
            // Trailing '$' — leave as-is.
            result.push('$');
            i += 1;
            continue;
        }

        if chars[next] == '$' {
            // '$$' → literal '$'
            result.push('$');
            i += 2;
            continue;
        }

        if chars[next] == '{' {
            // ${name} form — scan for closing '}'
            let start = next + 1;
            let mut end = start;
            while end < chars.len() && chars[end] != '}' {
                end += 1;
            }
            if end < chars.len() {
                // Valid ${name}
                let name: String = chars[start..end].iter().collect();
                if let Some(val) = vars.get(&name) {
                    result.push_str(val);
                } else {
                    // Unknown — leave as-is
                    result.push('$');
                    result.push('{');
                    result.push_str(&name);
                    result.push('}');
                }
                i = end + 1;
            } else {
                // No closing '}' — leave as-is
                result.push('$');
                i += 1;
            }
            continue;
        }

        if chars[next].is_alphabetic() || chars[next] == '_' {
            // $name form — scan identifier characters
            let start = next;
            let mut end = start;
            while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                end += 1;
            }
            let name: String = chars[start..end].iter().collect();
            if let Some(val) = vars.get(&name) {
                result.push_str(val);
            } else {
                result.push('$');
                result.push_str(&name);
            }
            i = end;
            continue;
        }

        // '$' followed by something that isn't a valid start — leave '$' as-is.
        result.push('$');
        i += 1;
    }

    result
}

/// Repeatedly call `safe_substitute` until the output stabilises or
/// `max_depth` iterations have been reached (replicating Python's
/// `reporters_db.utils.recursive_substitute`).
///
/// Returns `Err` if `max_depth` is exhausted (matches Python's `ValueError`).
pub fn recursive_substitute(
    template: &str,
    vars: &HashMap<String, String>,
    max_depth: usize,
) -> Result<String, String> {
    let mut current = template.to_owned();
    for _ in 0..max_depth {
        let next = safe_substitute(&current, vars);
        if next == current {
            return Ok(current);
        }
        current = next;
    }
    Err(format!(
        "max_depth ({max_depth}) exceeded for template '{template}'"
    ))
}

/// Flatten and resolve the raw JSON from `regexes.json`, replicating
/// `reporters_db.utils.process_variables`.
///
/// Input shape:
/// ```json
/// {
///   "volume": { "": "(?P<volume>[1-9]\\d*)", "year": "(?P<volume>...)" },
///   "paragraph_marker": "(?:P|¶|para?\\.)",
///   ...
/// }
/// ```
///
/// Processing steps (matching Python exactly):
/// 1. Flatten nested objects — join keys with `_`, the `""` key becomes the
///    bare parent name.  Keys ending in `#` are stripped (comments).
/// 2. For every key `k`, add `k_optional` = `(?:VALUE ?)?`.
/// 3. Recursively resolve `$variable` references using the full variable map.
pub fn process_variables(raw: &serde_json::Value) -> HashMap<String, String> {
    // Step 1: flatten
    let mut flat: HashMap<String, String> = HashMap::new();
    flatten_value(raw, "", &mut flat);

    // Step 2: add _optional variants (iterate over a snapshot to avoid
    // mutating while iterating)
    let keys: Vec<String> = flat.keys().cloned().collect();
    for k in keys {
        let v = flat[&k].clone();
        flat.insert(format!("{k}_optional"), format!("(?:{v} ?)?"));
    }

    // Step 3: resolve references
    let snapshot: HashMap<String, String> = flat.clone();
    flat.iter_mut().for_each(|(_k, v)| {
        *v = recursive_substitute(v, &snapshot, 100)
            .expect("process_variables: recursive_substitute exceeded max_depth");
    });

    flat
}

/// Recursive helper for `process_variables` step 1.
fn flatten_value(value: &serde_json::Value, parent_key: &str, out: &mut HashMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                // Strip comment keys
                if k.ends_with('#') {
                    continue;
                }
                // Build new key: join parent and child with '_', skipping empty parts
                let new_key = match (parent_key, k.as_str()) {
                    ("", "") => String::new(),
                    ("", child) => child.to_owned(),
                    (parent, "") => parent.to_owned(),
                    (parent, child) => format!("{parent}_{child}"),
                };
                flatten_value(v, &new_key, out);
            }
        }
        serde_json::Value::String(s) => {
            if !parent_key.ends_with('#') {
                out.insert(parent_key.to_owned(), s.clone());
            }
        }
        other => {
            // Numbers, bools, etc. — convert to string representation
            if !parent_key.ends_with('#') {
                out.insert(parent_key.to_owned(), other.to_string());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    // --- safe_substitute ---

    #[test]
    fn test_safe_substitute_basic() {
        let vars = HashMap::from([("foo".to_string(), "bar".to_string())]);
        assert_eq!(safe_substitute("$foo baz", &vars), "bar baz");
    }

    #[test]
    fn test_safe_substitute_braces() {
        let vars = HashMap::from([("foo".to_string(), "bar".to_string())]);
        assert_eq!(safe_substitute("${foo} baz", &vars), "bar baz");
    }

    #[test]
    fn test_safe_substitute_unknown_left_alone() {
        let vars = HashMap::new();
        assert_eq!(safe_substitute("$unknown", &vars), "$unknown");
    }

    #[test]
    fn test_safe_substitute_unknown_braces_left_alone() {
        let v = vars(&[]);
        assert_eq!(safe_substitute("${unknown} stays", &v), "${unknown} stays");
    }

    #[test]
    fn test_safe_substitute_dollar_dollar() {
        let v = vars(&[("foo", "bar")]);
        assert_eq!(safe_substitute("$$foo", &v), "$foo");
    }

    #[test]
    fn test_safe_substitute_trailing_dollar() {
        let v = vars(&[]);
        assert_eq!(safe_substitute("hello$", &v), "hello$");
    }

    // --- recursive_substitute ---

    #[test]
    fn test_recursive_substitute() {
        let vars = HashMap::from([
            ("a".to_string(), "$b".to_string()),
            ("b".to_string(), "resolved".to_string()),
        ]);
        assert_eq!(recursive_substitute("$a", &vars, 100).unwrap(), "resolved");
    }

    #[test]
    fn test_recursive_substitute_already_stable() {
        let v = vars(&[("a", "hello")]);
        assert_eq!(recursive_substitute("$a world", &v, 100).unwrap(), "hello world");
    }

    #[test]
    fn test_recursive_substitute_no_vars() {
        let v = vars(&[]);
        assert_eq!(recursive_substitute("literal", &v, 100).unwrap(), "literal");
    }

    #[test]
    fn test_recursive_substitute_max_depth_error() {
        // A → $B → $A (cycle) should hit max_depth
        let v = vars(&[("A", "$B"), ("B", "$A")]);
        let result = recursive_substitute("$A", &v, 5);
        assert!(result.is_err(), "expected Err for cyclic substitution");
    }

    // --- process_variables ---

    #[test]
    fn test_process_variables_flattening_and_optional() {
        let raw = serde_json::json!({
            "volume": {
                "": "(?P<volume>\\d+)",
                "year": "(?P<volume_year>\\d{4})",
                "#": "comment — should be dropped"
            }
        });

        let result = process_variables(&raw);

        // Flat keys
        assert!(result.contains_key("volume"), "missing 'volume'");
        assert!(result.contains_key("volume_year"), "missing 'volume_year'");
        // Comment key stripped
        assert!(!result.contains_key("volume_#"), "comment key present");
        // Optional variants
        assert!(result.contains_key("volume_optional"), "missing 'volume_optional'");
        assert!(result.contains_key("volume_year_optional"), "missing 'volume_year_optional'");

        // Check optional wrapping
        let vol = result.get("volume").unwrap();
        let vol_opt = result.get("volume_optional").unwrap();
        assert_eq!(vol_opt, &format!("(?:{vol} ?)?"));
    }

    #[test]
    fn test_process_variables_top_level_string() {
        let raw = serde_json::json!({
            "paragraph_marker": "(?:P|¶|para?\\.)"
        });
        let result = process_variables(&raw);
        assert_eq!(result.get("paragraph_marker").unwrap(), "(?:P|¶|para?\\.)");
    }

    #[test]
    fn test_process_variables_resolves_references() {
        let raw = serde_json::json!({
            "page": {
                "": "(?P<page>\\d+)"
            },
            "volume": {
                "": "(?P<volume>\\d+)"
            },
            "full_cite": {
                "": "$volume $page"
            }
        });
        let result = process_variables(&raw);
        let full_cite = result.get("full_cite").unwrap();
        assert!(
            full_cite.contains("(?P<volume>\\d+)"),
            "full_cite should contain resolved volume: {full_cite}"
        );
        assert!(
            full_cite.contains("(?P<page>\\d+)"),
            "full_cite should contain resolved page: {full_cite}"
        );
    }

    #[test]
    fn test_process_variables_from_json() {
        // Load actual regexes.json and verify output count
        let data: serde_json::Value = serde_json::from_str(
            include_str!("../../../data/reporters-db/reporters_db/data/regexes.json")
        ).unwrap();
        let result = process_variables(&data);
        // 7 top-level keys produce 33 base variables + 33 optional = 66 total
        assert_eq!(result.len(), 66);
        // Verify specific known outputs
        assert!(result.get("volume").unwrap().starts_with("(?P<volume>"));
        assert!(result.get("page").unwrap().starts_with("(?P<page>"));
        assert!(result.get("full_cite").unwrap().contains("(?P<volume>"));
        // Optional variants should wrap in (?:...)?
        assert!(result.get("volume_optional").unwrap().starts_with("(?:"));
        assert!(result.get("volume_optional").unwrap().ends_with("?)?"));
        // No unresolved $references (except $edition which is special)
        for (k, v) in &result {
            for m in v.match_indices('$') {
                let rest = &v[m.0..];
                // Only $edition should remain unresolved
                if !rest.starts_with("$edition") && !rest.starts_with("$$") {
                    // Check if it's a known variable reference that wasn't resolved
                    // (bare $ at end of string or $ followed by non-identifier is ok)
                    let after_dollar = &rest[1..];
                    if !after_dollar.is_empty() {
                        let first_char = after_dollar.chars().next().unwrap();
                        if first_char.is_alphabetic() || first_char == '_' {
                            panic!("Unresolved variable in {}: {}", k, rest.split_whitespace().next().unwrap_or(rest));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_process_variables_actual_regexes_json() {
        let raw: serde_json::Value = serde_json::from_str(include_str!(
            "../../../data/reporters-db/reporters_db/data/regexes.json"
        ))
        .expect("parse regexes.json");

        // Should have 7 top-level keys in the raw JSON
        let top_keys = raw.as_object().unwrap().len();
        assert_eq!(top_keys, 7, "expected 7 top-level keys in regexes.json, got {top_keys}");

        let vars = process_variables(&raw);
        assert_eq!(vars.len(), 66, "expected 66 processed variables, got {}", vars.len());
        assert!(vars.contains_key("full_cite"), "missing 'full_cite'");
        assert!(vars.contains_key("page"), "missing 'page'");
        assert!(vars.contains_key("volume"), "missing 'volume'");
    }
}
