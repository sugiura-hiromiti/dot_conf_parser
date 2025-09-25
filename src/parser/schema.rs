use crate::error::PRslt;
use crate::parser::conf::SingleValueDiscriminants;
use crate::parser::conf::Value;
use crate::parser::conf::ValueDiscriminants;
use crate::parser::core::StructuredInput;
use crate::parser::core::TreeValue;
use crate::parser::core::Valuable;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Default,)]
pub struct SchemaMap(BTreeMap<String, SchemaValue,>,);

impl SchemaMap {
	pub fn new() -> Self {
		Self(BTreeMap::new(),)
	}

	pub fn from_inner(inner: BTreeMap<String, SchemaValue,>,) -> Self {
		Self(inner,)
	}

	pub fn into_inner(self,) -> BTreeMap<String, SchemaValue,> {
		self.0
	}

	pub fn is_empty(&self,) -> bool {
		self.0.is_empty()
	}

	pub fn get(&self, key: &str,) -> Option<&SchemaValue,> {
		if let Some(value,) = self.0.get(key,) {
			return Some(value,);
		}

		let mut segments = key.split('.',);
		let first = segments.next()?;
		let mut current = self.0.get(first,)?;

		for segment in segments {
			current = match current {
				SchemaValue::Map(children,) => children.get(segment,)?,
				_ => return None,
			};
		}

		Some(current,)
	}
}

impl From<BTreeMap<String, SchemaValue,>,> for SchemaMap {
	fn from(inner: BTreeMap<String, SchemaValue,>,) -> Self {
		Self(inner,)
	}
}

impl<const N: usize,> From<[(String, SchemaValue,); N],> for SchemaMap {
	fn from(entries: [(String, SchemaValue,); N],) -> Self {
		Self(entries.into_iter().collect(),)
	}
}

impl Deref for SchemaMap {
	type Target = BTreeMap<String, SchemaValue,>;

	fn deref(&self,) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for SchemaMap {
	fn deref_mut(&mut self,) -> &mut Self::Target {
		&mut self.0
	}
}

pub type SchemaValue = TreeValue<Value<SingleValueDiscriminants,>,>;

impl Display for ValueDiscriminants {
	/// required by `ParseError`
	fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result {
		match self {
			Self::Single => write!(f, "Single"),
			Self::Collection => write!(f, "Collection"),
		}
	}
}

impl Valuable for SingleValueDiscriminants {
	fn sep() -> &'static str {
		"->"
	}
}

impl Display for SingleValueDiscriminants {
	/// required by `ParseError`
	fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result {
		match self {
			Self::String => write!(f, "String"),
			Self::Bool => write!(f, "Bool"),
			Self::Integer => write!(f, "Integer"),
		}
	}
}

pub fn parse_file<P: AsRef<Path,>,>(path: P,) -> PRslt<SchemaMap,> {
	let mir = crate::parser::core::file_to_mir::<_, SingleValueDiscriminants,>(
		path,
	)?;
	mir.into_schema()
}

pub fn parse_str(input: &str,) -> PRslt<SchemaMap,> {
	let mir =
		crate::parser::core::str_to_mir::<SingleValueDiscriminants,>(input,)?;
	mir.into_schema()
}

pub trait BuildSchema {
	fn into_schema(self,) -> PRslt<SchemaMap,>;
}

impl BuildSchema for StructuredInput {
	fn into_schema(self,) -> PRslt<SchemaMap,> {
		let mut schema_map = BTreeMap::new();

		for (key, mir_value,) in self.into_iter() {
			let schema = match mir_value {
				TreeValue::Scalar((s, _,),) => parse_schema_value(&s,)?,
				TreeValue::Map(btree_map,) => {
					TreeValue::Map(btree_map.into_schema()?.into_inner(),)
				},
			};

			schema_map.insert(key, schema,);
		}

		Ok(SchemaMap::from_inner(schema_map,),)
	}
}

fn parse_schema_value(value: &str,) -> PRslt<SchemaValue,> {
	Ok(TreeValue::Scalar(
		if value.contains(',',) {
			Value::Collection(
				value
					.split(',',)
					.map(|s| SingleValueDiscriminants::from_str(s.trim(),),)
					.try_collect()?,
			)
		} else {
			Value::Single(SingleValueDiscriminants::from_str(value,)?,)
		},
	),)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn scalar_line(value: &str, line: usize,) -> TreeValue<(String, usize,),> {
		TreeValue::Scalar((value.to_string(), line,),)
	}

	#[test]
	fn parse_schema_value_accepts_single_discriminant() {
		let schema = parse_schema_value("Bool",).unwrap();
		match schema {
			TreeValue::Scalar(Value::Single(kind,),) => {
				assert_eq!(kind, SingleValueDiscriminants::Bool);
			},
			other => panic!("unexpected schema value: {other:?}"),
		}
	}

	#[test]
	fn parse_schema_value_supports_collections() {
		let schema = parse_schema_value("Integer, Integer",).unwrap();
		match schema {
			TreeValue::Scalar(Value::Collection(kinds,),) => {
				assert_eq!(kinds.len(), 2);
				assert!(
					kinds.iter().all(|k| matches!(
						k,
						SingleValueDiscriminants::Integer
					))
				);
			},
			other => panic!("unexpected schema value: {other:?}"),
		}
	}

	#[test]
	fn into_schema_converts_nested_entries() {
		let mut mir = StructuredInput::new();
		mir.insert("flag".into(), scalar_line("Bool", 1,),);

		let mut nested_map = StructuredInput::new();
		nested_map.insert("port".into(), scalar_line("Integer", 2,),);
		mir.insert("server".into(), TreeValue::Map(nested_map,),);

		let schema = mir.into_schema().unwrap();

		match schema.get("flag",).unwrap() {
			TreeValue::Scalar(Value::Single(kind,),) => {
				assert_eq!(*kind, SingleValueDiscriminants::Bool);
			},
			other => panic!("unexpected flag schema: {other:?}"),
		}

		match schema.get("server",).unwrap() {
			TreeValue::Map(children,) => match children.get("port",).unwrap() {
				TreeValue::Scalar(Value::Single(kind,),) => {
					assert_eq!(*kind, SingleValueDiscriminants::Integer);
				},
				other => panic!("unexpected port schema: {other:?}"),
			},
			other => panic!("unexpected server schema: {other:?}"),
		}
	}

	#[test]
	fn parse_str_builds_schema_tree() {
		let schema = parse_str(
			"flag -> Bool\nserver.port -> Integer\nserver.host -> String",
		)
		.unwrap();

		assert!(matches!(
			schema.get("flag"),
			Some(TreeValue::Scalar(Value::Single(
				SingleValueDiscriminants::Bool
			)))
		));

		let server = schema.get("server",).unwrap();
		match server {
			TreeValue::Map(children,) => {
				assert!(matches!(
					children.get("port"),
					Some(TreeValue::Scalar(Value::Single(
						SingleValueDiscriminants::Integer
					)))
				));
				assert!(matches!(
					children.get("host"),
					Some(TreeValue::Scalar(Value::Single(
						SingleValueDiscriminants::String
					)))
				));
			},
			other => panic!("unexpected server schema: {other:?}"),
		}
	}

	#[test]
	fn display_for_value_discriminants_matches_variant_names() {
		assert_eq!(ValueDiscriminants::Single.to_string(), "Single");
		assert_eq!(ValueDiscriminants::Collection.to_string(), "Collection");
	}

	#[test]
	fn display_for_single_value_discriminants_lists_type_names() {
		assert_eq!(SingleValueDiscriminants::Bool.to_string(), "Bool");
		assert_eq!(SingleValueDiscriminants::String.to_string(), "String");
		assert_eq!(SingleValueDiscriminants::Integer.to_string(), "Integer");
	}
}
