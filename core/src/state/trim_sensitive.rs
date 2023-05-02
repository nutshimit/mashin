/* -------------------------------------------------------- *\
 *                                                          *
 *      ███╗░░░███╗░█████╗░░██████╗██╗░░██╗██╗███╗░░██╗     *
 *      ████╗░████║██╔══██╗██╔════╝██║░░██║██║████╗░██║     *
 *      ██╔████╔██║███████║╚█████╗░███████║██║██╔██╗██║     *
 *      ██║╚██╔╝██║██╔══██║░╚═══██╗██╔══██║██║██║╚████║     *
 *      ██║░╚═╝░██║██║░░██║██████╔╝██║░░██║██║██║░╚███║     *
 *      ╚═╝░░░░░╚═╝╚═╝░░╚═╝╚═════╝░╚═╝░░╚═╝╚═╝╚═╝░░╚══╝     *
 *                                         by Nutshimit     *
 * -------------------------------------------------------- *
 *                                                          *
 *  This file is licensed as MIT. See LICENSE for details.  *
 *                                                          *
\* ---------------------------------------------------------*/

use crate::sdk::{KEYS_CORE, KEY_CONFIG, KEY_SENSITIVE, KEY_URN, KEY_VALUE};
use mashin_sdk::KEY_NAME;
use serde_json::Value;

pub fn fold_json(json_obj: &Value, replace_sensitive: Option<&str>) -> Value {
	if let Value::Object(map) = json_obj {
		let mut new_obj = serde_json::Map::new();
		let mut should_skip_sensitive = false;

		if let Some(sensitive_val) = map.get(KEY_SENSITIVE) {
			should_skip_sensitive = sensitive_val.as_bool().unwrap_or(false);

			if should_skip_sensitive {
				if let Some(replace_sensitive) = replace_sensitive {
					if let Some(key_val) = map.get(KEY_VALUE) {
						if key_val.is_object() {
							replace_secrets_string(&mut key_val.clone(), replace_sensitive);
							return key_val.clone()
						} else {
							return replace_sensitive.into()
						}
					} else {
						return replace_sensitive.into()
					}
				}
			}
		}

		if !should_skip_sensitive {
			for (key, value) in map {
				let mut key = key.clone();
				if KEYS_CORE.contains(&key.as_str()) {
					continue
				}

				if key == KEY_VALUE {
					return value.clone()
				}

				if key == KEY_CONFIG {
					key = "config".to_string();
				}

				if key == KEY_URN {
					key = "urn".to_string();
				}

				if key == KEY_NAME {
					key = "name".to_string();
				}

				if value.is_object() {
					let processed_value = fold_json(value, replace_sensitive);
					if !processed_value.is_null() {
						new_obj.insert(key.clone(), processed_value);
					}
				} else {
					new_obj.insert(key.clone(), value.clone());
				}
			}
		}

		if new_obj.is_empty() {
			Value::Null
		} else {
			Value::Object(new_obj)
		}
	} else {
		json_obj.clone()
	}
}

// replace all string value to `sensitive_str` value
fn replace_secrets_string(value: &mut Value, sensitive_str: &str) {
	match value {
		Value::Object(ref mut map) =>
			for (_, v) in map.iter_mut() {
				replace_secrets_string(v, sensitive_str);
			},
		Value::Array(ref mut arr) =>
			for v in arr.iter_mut() {
				replace_secrets_string(v, sensitive_str);
			},
		Value::String(_) => {
			*value = Value::String(sensitive_str.to_string());
		},
		_ => {},
	}
}
