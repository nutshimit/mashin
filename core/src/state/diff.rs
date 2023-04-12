use std::collections::HashSet;
use std::fmt;
use std::ops::Deref;

use itertools::merge_join_by;
use itertools::EitherOrBoth::{Both, Left, Right};
use mashin_sdk::{ResourceDiff, KEYS_CORE, KEY_VALUE};
use serde_json::Value;

pub trait Indexes {
    fn indexes(&self) -> Vec<usize>;
}

impl<T> Indexes for Vec<T> {
    fn indexes(&self) -> Vec<usize> {
        if self.is_empty() {
            vec![]
        } else {
            (0..=self.len() - 1).collect()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Path {
    Root,
    Keys(Vec<Key>),
}

impl Path {
    fn append(&self, next: Key) -> Path {
        match self {
            Path::Root => Path::Keys(vec![next]),
            Path::Keys(list) => {
                let mut copy = list.clone();
                copy.push(next);
                Path::Keys(copy)
            }
        }
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // if state are equal, there is no diff generated
            Path::Root => unimplemented!(),
            // grab all keys
            Path::Keys(keys) => {
                write!(
                    f,
                    "{}",
                    keys.iter()
                        .filter_map(|s| {
                            let key_str = s.to_string();
                            if &key_str == KEY_VALUE {
                                None
                            } else {
                                Some(key_str.replace("__config", "config"))
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(".")
                )
            }
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct StateDiff {
    resources: Vec<StateResourceDiff>,
}

impl Deref for StateDiff {
    type Target = Vec<StateResourceDiff>;

    fn deref(&self) -> &Self::Target {
        &self.resources
    }
}

impl StateDiff {
    pub fn provider_resource_diff(&self) -> ResourceDiff {
        ResourceDiff::new(self.resources.iter().map(|s| s.path.to_string()).collect())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateResourceDiff {
    path: Path,
    lhs: Option<Value>,
    rhs: Option<Value>,
}

impl StateResourceDiff {
    pub fn is_eq(&self) -> bool {
        self.lhs == self.rhs
    }

    pub fn is_create(&self) -> bool {
        self.rhs.is_none()
    }

    pub fn is_delete(&self) -> bool {
        self.lhs.is_none()
    }

    pub fn is_update(&self) -> bool {
        self.lhs.is_some() && self.rhs.is_some()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn previous_state(&self) -> &Option<Value> {
        &self.rhs
    }

    pub fn new_state(&self) -> &Option<Value> {
        &self.lhs
    }
}

#[derive(Debug)]
struct DiffFolder<'a> {
    rhs: Value,
    path: Path,
    acc: &'a mut Vec<StateResourceDiff>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    Idx(usize),
    Field(String),
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Key::Idx(idx) => write!(f, "[{}]", idx),
            Key::Field(key) => write!(f, "{}", key),
        }
    }
}

pub fn diff(lhs: Value, rhs: Value) -> StateDiff {
    let mut acc = vec![];
    diff_with(lhs, rhs, Path::Root, &mut acc);
    StateDiff { resources: acc }
}

fn diff_with(lhs: Value, rhs: Value, path: Path, acc: &mut Vec<StateResourceDiff>) {
    let mut folder = DiffFolder { rhs, path, acc };
    fold_json(lhs, &mut folder);
}

fn fold_json(json: Value, folder: &mut DiffFolder<'_>) {
    match json {
        Value::Null => folder.on_null(json),
        Value::Bool(_) => folder.on_bool(json),
        Value::Number(_) => folder.on_number(json),
        Value::String(_) => folder.on_string(json),
        Value::Array(_) => folder.on_array(json),
        Value::Object(_) => folder.on_object(json),
    }
}

macro_rules! direct_compare {
    ($name:ident) => {
        fn $name(&mut self, lhs: Value) {
            if self.rhs != lhs {
                self.acc.push(StateResourceDiff {
                    lhs: Some(lhs),
                    rhs: Some(self.rhs.clone()),
                    path: self.path.clone(),
                });
            }
        }
    };
}

impl<'a> DiffFolder<'a> {
    direct_compare!(on_null);
    direct_compare!(on_bool);
    direct_compare!(on_string);

    fn on_number(&mut self, lhs: Value) {
        let is_equal = self.rhs == lhs;
        if !is_equal {
            self.acc.push(StateResourceDiff {
                lhs: Some(lhs),
                rhs: Some(self.rhs.clone()),
                path: self.path.clone(),
            });
        }
    }

    fn on_array(&mut self, lhs: Value) {
        if let Some(rhs) = self.rhs.as_array() {
            let lhs = lhs.as_array().unwrap();

            let all_keys = rhs
                .indexes()
                .into_iter()
                .chain(lhs.indexes())
                .collect::<HashSet<_>>();
            for key in all_keys {
                let path = self.path.append(Key::Idx(key));

                match (lhs.get(key), rhs.get(key)) {
                    (Some(lhs), Some(rhs)) => {
                        diff_with(lhs.clone(), rhs.clone(), path, self.acc);
                    }
                    (None, Some(rhs)) => {
                        self.acc.push(StateResourceDiff {
                            lhs: None,
                            rhs: Some(rhs.clone()),
                            path,
                        });
                    }
                    (Some(lhs), None) => {
                        self.acc.push(StateResourceDiff {
                            lhs: Some(lhs.clone()),
                            rhs: None,
                            path,
                        });
                    }
                    (None, None) => {
                        unreachable!("at least one of the maps should have the key")
                    }
                }
            }
        } else {
            self.acc.push(StateResourceDiff {
                lhs: Some(lhs),
                rhs: Some(self.rhs.clone()),
                path: self.path.clone(),
            });
        }
    }

    fn on_object(&mut self, lhs: Value) {
        if let Some(rhs) = self.rhs.as_object() {
            let lhs = lhs.as_object().unwrap();

            let all_keys = rhs.keys().chain(lhs.keys()).collect::<HashSet<_>>();
            for key in all_keys {
                let path = self.path.append(Key::Field(key.clone()));

                match (lhs.get(key), rhs.get(key)) {
                    (Some(lhs), Some(rhs)) => {
                        diff_with(lhs.clone(), rhs.clone(), path, self.acc);
                    }
                    (None, Some(rhs)) => {
                        self.acc.push(StateResourceDiff {
                            lhs: None,
                            rhs: Some(rhs.clone()),
                            path,
                        });
                    }
                    (Some(lhs), None) => {
                        self.acc.push(StateResourceDiff {
                            lhs: Some(lhs.clone()),
                            rhs: None,
                            path,
                        });
                    }
                    (None, None) => {
                        unreachable!("at least one of the maps should have the key")
                    }
                }
            }
        } else {
            self.acc.push(StateResourceDiff {
                lhs: Some(lhs),
                rhs: Some(self.rhs.clone()),
                path: self.path.clone(),
            });
        }
    }
}
