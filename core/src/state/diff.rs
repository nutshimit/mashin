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

use super::trim_sensitive::fold_json;
use crate::Result;
use console::style;
use mashin_sdk::{ResourceDiff, KEY_VALUE};
use serde_json::Value;
use std::{collections::HashSet, fmt, ops::Deref};

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
			},
		}
	}
}

impl fmt::Display for Path {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			// if state are equal, there is no diff generated
			Path::Root => write!(f, "TEST"),
			// grab all keys
			Path::Keys(keys) => {
				write!(
					f,
					"{}",
					keys.iter()
						.filter_map(|s| {
							let key_str = s.to_string();
							if key_str == KEY_VALUE {
								None
							} else {
								Some(key_str.replace("__config", "config"))
							}
						})
						.collect::<Vec<_>>()
						.join(".")
				)
			},
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
		self.rhs_is_null() && !self.lhs_is_null()
	}

	pub fn is_delete(&self) -> bool {
		self.lhs_is_null() && !self.rhs_is_null()
	}

	pub fn is_update(&self) -> bool {
		!self.lhs_is_null() && !self.rhs_is_null()
	}

	/// The diff path
	pub fn path(&self) -> &Path {
		&self.path
	}

	/// Habitually the previous state
	pub fn rhs(&self) -> &Option<Value> {
		&self.rhs
	}

	/// Habitually the new state
	pub fn lhs(&self) -> &Option<Value> {
		&self.lhs
	}

	/// Check if lhs (habitually the new state) is null
	pub fn lhs_is_null(&self) -> bool {
		match &self.lhs {
			Some(lhs) => lhs.is_null(),
			None => false,
		}
	}

	/// Check if rhs (habitually the previous state) is null
	pub fn rhs_is_null(&self) -> bool {
		match &self.rhs {
			Some(rhs) => rhs.is_null(),
			None => false,
		}
	}

	// return the closing line if needed with the right color
	pub fn print_diff(&self) -> Result<Option<String>> {
		const LINE: &str = "-------------------\n\n";
		if self.is_create() {
			let mut diff_print = self.lhs().clone().unwrap_or_default().to_string();

			if let Some(new_state) = self.lhs() {
				if new_state.is_object() {
					diff_print = mashin_sdk::ext::serde_json::to_string_pretty(new_state)?
						.split('\n')
						.collect::<Vec<_>>()
						.join("\n   |     + ");
				}
			}
			log::info!(
				"   {}     {} {}: {}",
				style("|").green().bold(),
				style("+").green().bold(),
				style(self.path().to_string()).green().bold(),
				style(diff_print).green().bold()
			);
			return Ok(Some(style(LINE).green().bold().to_string()))
		}

		if self.is_update() {
			let mut diff_new = style(self.lhs().clone().unwrap_or_default().to_string())
				.green()
				.bold()
				.to_string();
			let mut diff_old = style(self.rhs().clone().unwrap_or_default().to_string())
				.red()
				.dim()
				.to_string();

			let whitespace = " ".repeat(self.path().to_string().len());
			let box_line = format!(
				"{}{}",
				style("   |     ",).cyan().bold(),
				style("^".repeat(self.path().to_string().len() + 3),).cyan().bold()
			);

			if let Some(new_state) = self.lhs() {
				if new_state.is_object() {
					diff_new = mashin_sdk::ext::serde_json::to_string_pretty(new_state)?
						.split('\n')
						.map(|s| style(s).green().bold().to_string())
						.collect::<Vec<_>>()
						.join(
							format!("{}  {}", style("\n   |     +").cyan().bold(), whitespace)
								.as_str(),
						);
				}
			}

			if let Some(old_state) = self.rhs() {
				if old_state.is_object() {
					diff_old = mashin_sdk::ext::serde_json::to_string_pretty(old_state)?
						.split('\n')
						.map(|s| style(s).red().dim().to_string())
						.collect::<Vec<_>>()
						.join(
							format!("{}  {}", style("\n   |     -").cyan().bold(), whitespace)
								.as_str(),
						);
				}
			}

			log::info!(
				"   {}     {} {}: {}",
				style("|").cyan().bold(),
				style("-").cyan().bold(),
				style(self.path().to_string()).cyan().bold(),
				diff_old
			);
			log::info!(
				"   {}     {} {}  {}",
				style("|").cyan().bold(),
				style("+").cyan().bold(),
				whitespace,
				diff_new
			);

			log::info!("{}", box_line);

			return Ok(Some(style(LINE).cyan().bold().to_string()))
		}

		Ok(None)
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
	diff_with(
		fold_json(&lhs, Some("[sensitive]")),
		fold_json(&rhs, Some("[sensitive]")),
		Path::Root,
		&mut acc,
	);
	StateDiff { resources: acc }
}

fn diff_with(lhs: Value, rhs: Value, path: Path, acc: &mut Vec<StateResourceDiff>) {
	let mut folder = DiffFolder { rhs, path, acc };
	match lhs {
		Value::Null => folder.on_null(lhs),
		Value::Bool(_) => folder.on_bool(lhs),
		Value::Number(_) => folder.on_number(lhs),
		Value::String(_) => folder.on_string(lhs),
		Value::Array(_) => folder.on_array(lhs),
		Value::Object(_) => folder.on_object(lhs),
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

			let all_keys = rhs.indexes().into_iter().chain(lhs.indexes()).collect::<HashSet<_>>();
			for key in all_keys {
				let path = self.path.append(Key::Idx(key));

				match (lhs.get(key), rhs.get(key)) {
					(Some(lhs), Some(rhs)) => {
						diff_with(lhs.clone(), rhs.clone(), path, self.acc);
					},
					(None, Some(rhs)) => {
						self.acc.push(StateResourceDiff {
							lhs: None,
							rhs: Some(rhs.clone()),
							path,
						});
					},
					(Some(lhs), None) => {
						self.acc.push(StateResourceDiff {
							lhs: Some(lhs.clone()),
							rhs: None,
							path,
						});
					},
					(None, None) => {
						unreachable!("at least one of the maps should have the key")
					},
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
					},
					(None, Some(rhs)) => {
						self.acc.push(StateResourceDiff {
							lhs: None,
							rhs: Some(rhs.clone()),
							path,
						});
					},
					(Some(lhs), None) => {
						self.acc.push(StateResourceDiff {
							lhs: Some(lhs.clone()),
							rhs: None,
							path,
						});
					},
					(None, None) => {
						unreachable!("at least one of the maps should have the key")
					},
				}
			}
		} else if self.path == Path::Root {
			let lhs = lhs.as_object().unwrap();
			for (key, value) in lhs {
				let path = self.path.append(Key::Field(key.clone()));
				diff_with(value.clone(), Value::Null, path, self.acc);
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
