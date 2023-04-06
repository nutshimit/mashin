use crate::sdk::{KEYS_CORE, KEY_SENSITIVE, KEY_VALUE};
use serde_json::Value;

pub fn trim_sensitive_fields(json_obj: &Value) -> Value {
    let mut new_obj = serde_json::Map::new();
    if let Value::Object(map) = json_obj {
        let mut sensitive = false;
        if let Some(sensitive_val) = map.get(KEY_SENSITIVE) {
            sensitive = sensitive_val.as_bool().unwrap_or(false);
        }

        if !sensitive {
            for (key, value) in map {
                if KEYS_CORE.contains(&key.as_str()) {
                    continue;
                }

                if key == KEY_VALUE {
                    return value.clone();
                }

                if let Value::Object(_) = value {
                    let processed_value = trim_sensitive_fields(value);
                    if !processed_value.is_null() {
                        new_obj.insert(key.clone(), processed_value);
                    }
                } else {
                    new_obj.insert(key.clone(), value.clone());
                }
            }
        }
    }
    if new_obj.is_empty() {
        Value::Null
    } else {
        Value::Object(new_obj)
    }
}
