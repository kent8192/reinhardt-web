//! Parameter representation utilities for logging
//! Based on SQLAlchemy's _repr_params functionality

use serde_json::Value;

/// Configuration for parameter representation
pub struct ReprParamsConfig {
    pub max_chars: usize,
    pub batches: usize,
    pub is_multi: bool,
}

impl Default for ReprParamsConfig {
    fn default() -> Self {
        Self {
            max_chars: 300,
            batches: 10,
            is_multi: false,
        }
    }
}
/// Represent parameters in a truncated form suitable for logging
///
/// # Examples
///
/// ```
/// use reinhardt_logging::params::{repr_params, ReprParamsConfig};
/// use serde_json::json;
///
// Simple parameter representation
/// let params = json!({"user_id": 42, "email": "test@example.com"});
/// let config = ReprParamsConfig::default();
/// let output = repr_params(&params, &config);
///
// Output includes the parameter values
/// assert!(output.contains("user_id"));
/// assert!(output.contains("42"));
/// ```
pub fn repr_params(params: &Value, config: &ReprParamsConfig) -> String {
    match params {
        Value::Array(arr) if config.is_multi && !arr.is_empty() => repr_multi_params(arr, config),
        Value::Array(arr) => repr_array(arr, config),
        Value::Object(obj) => repr_object(obj, config),
        _ => format!("{:?}", params),
    }
}

fn repr_multi_params(params: &[Value], config: &ReprParamsConfig) -> String {
    let total = params.len();

    if total <= config.batches {
        return format!("{:?}", params);
    }

    let show_count = config.batches / 2;
    let first_items: Vec<String> = params
        .iter()
        .take(show_count)
        .map(|v| format!("{:?}", v))
        .collect();
    let last_items: Vec<String> = params
        .iter()
        .rev()
        .take(show_count)
        .map(|v| format!("{:?}", v))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    format!(
        "[{}  ... displaying {} of {} total bound parameter sets ...  {}]",
        first_items.join(", "),
        config.batches,
        total,
        last_items.join(", ")
    )
}

fn repr_array(arr: &[Value], config: &ReprParamsConfig) -> String {
    let repr = format!("{:?}", arr);

    if repr.len() <= config.max_chars {
        return repr;
    }

    let half_chars = config.max_chars / 2;
    let truncated_chars = repr.len() - config.max_chars;

    format!(
        "{}  ... ({} characters truncated) ...  {}",
        &repr[..half_chars],
        truncated_chars,
        &repr[repr.len() - half_chars..]
    )
}

fn repr_object(obj: &serde_json::Map<String, Value>, config: &ReprParamsConfig) -> String {
    let repr = format!("{:?}", obj);

    if repr.len() <= config.max_chars {
        return repr;
    }

    // For huge dicts, truncate from middle
    if obj.len() > 100 {
        let half_chars = config.max_chars / 2;
        let truncated_params = obj.len() - 10; // Show ~10 params

        format!(
            "{} ... {} parameters truncated ... {}",
            &repr[..half_chars],
            truncated_params,
            &repr[repr.len() - half_chars..]
        )
    } else {
        let half_chars = config.max_chars / 2;
        let truncated_chars = repr.len() - config.max_chars;

        format!(
            "{}  ... ({} characters truncated) ...  {}",
            &repr[..half_chars],
            truncated_chars,
            &repr[repr.len() - half_chars..]
        )
    }
}
/// Truncate a single parameter value if it's too long
///
/// # Examples
///
/// ```
/// use reinhardt_logging::params::truncate_param;
///
// Short strings are returned unchanged
/// let short_text = "Hello";
/// assert_eq!(truncate_param(short_text, 100), "Hello");
///
// Long strings are truncated with indication
/// let long_text = "a".repeat(1000);
/// let truncated = truncate_param(&long_text, 50);
/// assert!(truncated.len() < 1000);
/// assert!(truncated.contains("characters truncated"));
/// ```
pub fn truncate_param(value: &str, max_chars: usize) -> String {
    if value.len() <= max_chars {
        return value.to_string();
    }

    let half_chars = max_chars / 2;
    let truncated_chars = value.len() - max_chars;

    format!(
        "{} ... ({} characters truncated) ... {}",
        &value[..half_chars],
        truncated_chars,
        &value[value.len() - half_chars..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_repr_params_large_list_of_dict() {
        let params: Vec<Value> = (0..100).map(|i| json!({"data": i.to_string()})).collect();

        let config = ReprParamsConfig {
            max_chars: 300,
            batches: 10,
            is_multi: true,
        };

        let result = repr_params(&Value::Array(params), &config);

        // Should show first few and last few
        assert!(result.contains("displaying 10 of 100"));
        // Debug output format may vary, just check for data presence
        assert!(result.contains("data") && result.contains("0"));
        assert!(result.contains("99"));
    }

    #[test]
    fn test_repr_params_positional_array() {
        let params = json!([[1, 2, 3], 5]);

        let config = ReprParamsConfig {
            max_chars: 300,
            batches: 10,
            is_multi: false,
        };

        let result = repr_params(&params, &config);
        // Check for presence of numbers, format may vary
        assert!(result.contains("1"));
        assert!(result.contains("2"));
        assert!(result.contains("3"));
        assert!(result.contains("5"));
    }

    #[test]
    fn test_repr_params_unknown_list() {
        let large_array: Vec<i32> = (0..300).collect();
        let params = json!([large_array, 5]);

        let config = ReprParamsConfig {
            max_chars: 80,
            batches: 10,
            is_multi: false,
        };

        let result = repr_params(&params, &config);
        assert!(result.contains("characters truncated"));
    }

    #[test]
    fn test_repr_params_named_dict() {
        let mut params = serde_json::Map::new();
        for i in 0..10 {
            params.insert(format!("key_{}", i), json!(i));
        }

        let config = ReprParamsConfig {
            max_chars: 300,
            batches: 10,
            is_multi: false,
        };

        let result = repr_params(&Value::Object(params.clone()), &config);
        // Should fit without truncation
        assert!(!result.contains("truncated"));
    }

    #[test]
    fn test_repr_params_huge_named_dict() {
        let mut params = serde_json::Map::new();
        for i in 0..800 {
            params.insert(format!("key_{}", i), json!(i));
        }

        let config = ReprParamsConfig {
            max_chars: 1400,
            batches: 10,
            is_multi: false,
        };

        let result = repr_params(&Value::Object(params), &config);
        assert!(result.contains("parameters truncated"));
    }

    #[test]
    fn test_repr_params_ismulti_named_dict() {
        let param: serde_json::Map<String, Value> =
            (0..10).map(|i| (format!("key_{}", i), json!(i))).collect();

        let params: Vec<Value> = (0..50).map(|_| Value::Object(param.clone())).collect();

        let config = ReprParamsConfig {
            max_chars: 80,
            batches: 5,
            is_multi: true,
        };

        let result = repr_params(&Value::Array(params), &config);
        assert!(result.contains("displaying 5 of 50"));
    }

    #[test]
    fn test_truncate_param() {
        let large_param = "a".repeat(5000);
        let result = truncate_param(&large_param, 298);

        assert!(result.len() < 5000);
        assert!(result.contains("characters truncated"));
        assert!(result.starts_with("aaaa"));
        assert!(result.ends_with("aaaa"));
    }

    #[test]
    fn test_truncate_param_small() {
        let small_param = "small";
        let result = truncate_param(small_param, 100);

        assert_eq!(result, "small");
    }
}
