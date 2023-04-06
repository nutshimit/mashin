use super::{KEYS_CORE, KEY_VALUE};
use itertools::merge_join_by;
use itertools::EitherOrBoth::{Both, Left, Right};

pub fn compare_json_objects_recursive(
    a: &serde_json::Value,
    b: &serde_json::Value,
    path: Option<&str>,
    in_value: bool,
) -> Vec<String> {
    let mut keys_with_diffs = Vec::new();
    let path = path.unwrap_or_default();
    match (a, b) {
        (serde_json::Value::Object(obj_a), serde_json::Value::Object(obj_b)) => {
            let sorted_a = obj_a.iter().collect::<Vec<(&String, &serde_json::Value)>>();
            let sorted_b = obj_b.iter().collect::<Vec<(&String, &serde_json::Value)>>();

            for diff in merge_join_by(sorted_a, sorted_b, |&(k1, _), &(k2, _)| k1.cmp(k2)) {
                match diff {
                    Left((k, v)) => {
                        if KEYS_CORE.contains(&k.as_str()) {
                            continue;
                        }
                        let is_value = k == KEY_VALUE;
                        let nested_path = join_path(path, k, is_value);
                        let removed_keys = collect_all_keys_with_prefix(v, &nested_path);
                        if removed_keys.is_empty() {
                            if !is_value || in_value {
                                keys_with_diffs.push(format!("- {}", nested_path));
                            }
                        } else {
                            for removed_key in removed_keys {
                                keys_with_diffs.push(format!("- {}", removed_key));
                            }
                        }
                    }
                    Right((k, v)) => {
                        if KEYS_CORE.contains(&k.as_str()) {
                            continue;
                        }
                        let is_value = k == KEY_VALUE;
                        let nested_path = join_path(path, k, is_value);
                        let added_keys = collect_all_keys_with_prefix(v, &nested_path);
                        if added_keys.is_empty() {
                            if !is_value || in_value {
                                keys_with_diffs.push(format!("+ {}", nested_path));
                            }
                        } else {
                            for added_key in added_keys {
                                keys_with_diffs.push(format!("+ {}", added_key));
                            }
                        }
                    }
                    Both((k1, v1), (_, v2)) => {
                        if KEYS_CORE.contains(&k1.as_str()) {
                            continue;
                        }
                        let is_value = k1 == KEY_VALUE;
                        let nested_path = join_path(path, k1, is_value);
                        let nested_diffs = compare_json_objects_recursive(
                            v1,
                            v2,
                            Some(&nested_path),
                            is_value || in_value,
                        );
                        keys_with_diffs.extend(nested_diffs);
                    }
                }
            }
        }
        _ => {
            if a != b && in_value {
                keys_with_diffs.push(format!("* {}", path));
            }
        }
    }

    keys_with_diffs
}

fn join_path(path: &str, segment: &str, is_value: bool) -> String {
    if is_value {
        format!(
            "{}{}",
            path,
            segment
                .strip_prefix(&format!("{}/", KEY_VALUE))
                .unwrap_or_default()
                .to_string()
        )
    } else if path.is_empty() {
        segment.to_string()
    } else {
        format!("{}/{}", path, segment)
    }
}

fn collect_all_keys_with_prefix(json_value: &serde_json::Value, prefix: &str) -> Vec<String> {
    let mut keys = Vec::new();
    match json_value {
        serde_json::Value::Object(obj) => {
            for (k, v) in obj {
                if KEYS_CORE.contains(&k.as_str()) {
                    continue;
                }
                if k == KEY_VALUE {
                    keys.push(prefix.to_string());
                } else {
                    let nested_prefix = format!("{}{}/", prefix, k);
                    keys.extend(collect_all_keys_with_prefix(v, &nested_prefix));
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let nested_prefix = format!("{}{}/", prefix, i);
                keys.extend(collect_all_keys_with_prefix(v, &nested_prefix));
            }
        }
        _ => {
            keys.push(prefix.to_string());
        }
    }
    keys
}
