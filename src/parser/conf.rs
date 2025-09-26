use crate::error::PRslt;
use crate::error::ParseError;
use crate::parser::core::StructuredInput;
use crate::parser::core::TreeValue;
use crate::parser::core::Valuable;
use crate::parser::schema::SchemaMap;
use crate::parser::schema::SchemaValue;
use std::collections::BTreeMap;
use std::ops::Deref;
use std::ops::DerefMut;
use std::path::Path;
use strum_macros::EnumString;

pub type ConfValue = TreeValue<Value<SingleValue,>,>;

#[derive(Debug, Default,)]
pub struct ConfMap(BTreeMap<String, ConfValue,>,);

impl ConfMap {
	pub fn new() -> Self {
		Self(BTreeMap::new(),)
	}

	pub fn into_inner(self,) -> BTreeMap<String, ConfValue,> {
		self.0
	}

	pub fn get(&self, key: &str,) -> Option<&ConfValue,> {
		if let Some(value,) = self.0.get(key,) {
			return Some(value,);
		}

		let mut segments = key.split('.',);
		let first = segments.next()?;
		let mut current = self.0.get(first,)?;

		for segment in segments {
			current = match current {
				ConfValue::Map(children,) => children.get(segment,)?,
				_ => return None,
			};
		}

		Some(current,)
	}
}

impl From<&BTreeMap<String, ConfValue,>,> for ConfMap {
	fn from(inner: &BTreeMap<String, ConfValue,>,) -> Self {
		let inner = inner
			.iter()
			.map(|(key, value,)| {
				// let value = match value {
				// 	TreeValue::Scalar(v,) => match v {
				// 		Value::Single(v,) => {
				// 			TreeValue::Scalar(Value::Single(v.clone(),),)
				// 		},
				// 		Value::Collection(items,) => {
				// 			TreeValue::Scalar(Value::Collection(items.clone(),),)
				// 		},
				// 	},
				// 	TreeValue::Map(btree_map,) => todo!(),
				// };
				(key.clone(), value.clone(),)
			},)
			.collect();
		Self(inner,)
	}
}

impl Deref for ConfMap {
	type Target = BTreeMap<String, ConfValue,>;

	fn deref(&self,) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for ConfMap {
	fn deref_mut(&mut self,) -> &mut Self::Target {
		&mut self.0
	}
}

#[derive(Debug, strum_macros::EnumDiscriminants, Clone,)]
pub enum Value<T: Valuable,> {
	Single(T,),
	Collection(Vec<T,>,),
}

#[derive(strum_macros::EnumDiscriminants, Debug, Clone, PartialEq, Eq,)]
#[strum_discriminants(derive(EnumString))]
pub enum SingleValue {
	String(String,),
	Bool(bool,),
	Integer(i32,),
}

impl Valuable for SingleValue {
	fn sep() -> &'static str {
		"="
	}
}

pub fn parse_file<P: AsRef<Path,>,>(
	path: P,
	schema_path: P,
) -> PRslt<ConfMap,> {
	let mir = crate::parser::core::file_to_mir::<_, SingleValue,>(path,)?;
	let schema = crate::parser::schema::parse_file(schema_path,)?;
	mir.into_conf(&schema,)
}

pub fn parse_str(input: &str, schema: SchemaMap,) -> PRslt<ConfMap,> {
	let mir = crate::parser::core::str_to_mir::<SingleValue,>(input,)?;
	mir.into_conf(&schema,)
}

pub trait BuildConf {
	fn into_conf(self, schema: &SchemaMap,) -> PRslt<ConfMap,>;
}

fn format_unknown_key_path(
	root: &str,
	value: &TreeValue<(String, usize,),>,
) -> String {
	let mut path = root.to_string();
	let mut current = value;

	while let TreeValue::Map(children,) = current {
		let Some((child_key, child_value,),) = children.iter().next() else {
			break;
		};

		if !path.is_empty() {
			path.push('.',);
		}

		path.push_str(child_key,);
		current = child_value;
	}

	path
}

trait SchemaLookup {
	fn lookup(&self, key: &str,) -> Option<&SchemaValue,>;
	fn is_empty(&self,) -> bool;
}

impl SchemaLookup for SchemaMap {
	fn lookup(&self, key: &str,) -> Option<&SchemaValue,> {
		self.get(key,)
	}

	fn is_empty(&self,) -> bool {
		self.is_empty()
	}
}

impl SchemaLookup for BTreeMap<String, SchemaValue,> {
	fn lookup(&self, key: &str,) -> Option<&SchemaValue,> {
		self.get(key,)
	}

	fn is_empty(&self,) -> bool {
		self.is_empty()
	}
}

fn build_conf_map<L: SchemaLookup + ?Sized,>(
	input: StructuredInput,
	schema: &L,
	prefix: Option<&str,>,
) -> PRslt<BTreeMap<String, ConfValue,>,> {
	let mut conf_map = BTreeMap::new();

	for (key, mir_value,) in input.into_iter() {
		let dotted_key = match prefix {
			Some(base,) => format!("{base}.{key}"),
			None => key.clone(),
		};

		let Some(schema_value,) = schema.lookup(&key,) else {
			if prefix.is_none() && !schema.is_empty() {
				return Err(ParseError::UnknownKey {
					key,
					lines: mir_value.get_lines_of_key(),
				},);
			}

			let unknown_key = format_unknown_key_path(&dotted_key, &mir_value,);
			return Err(ParseError::UnknownKey {
				key:   unknown_key,
				lines: mir_value.get_lines_of_key(),
			},);
		};

		let conf_value = match schema_value {
			TreeValue::Scalar(schema_value,) => {
				inject_payload(&dotted_key, schema_value, mir_value,)?
			},
			TreeValue::Map(schema_map,) => {
				let TreeValue::Map(nested_input,) = mir_value else { todo!() };
				let nested = build_conf_map(
					nested_input,
					schema_map,
					Some(&dotted_key,),
				)?;
				TreeValue::Map(nested,)
			},
		};

		conf_map.insert(key, conf_value,);
	}

	Ok(conf_map,)
}

impl BuildConf for StructuredInput {
	fn into_conf(self, schema: &SchemaMap,) -> PRslt<ConfMap,> {
		let conf_map = build_conf_map(self, schema, None,)?;
		Ok(ConfMap::from(&conf_map,),)
	}
}

impl SingleValueDiscriminants {
	fn into_payload(
		self,
		key: &str,
		value: &str,
		line: usize,
	) -> PRslt<SingleValue,> {
		Ok(match self {
			Self::String => SingleValue::String(value.to_string(),),
			Self::Bool => SingleValue::Bool(value == "true",),
			Self::Integer => {
				SingleValue::Integer(parse_str_as_i32(key, value, line,)?,)
			},
		},)
	}
}

fn parse_str_as_i32(key: &str, value: &str, line: usize,) -> PRslt<i32,> {
	value.parse::<i32>().map_err(|_| ParseError::InvalidValue {
		key: key.to_string(),
		value: value.to_string(),
		ty: SingleValueDiscriminants::Integer,
		line,
	},)
}

fn inject_payload(
	key: &str,
	schema_value: &Value<SingleValueDiscriminants,>,
	mir_value: TreeValue<(String, usize,),>,
) -> PRslt<ConfValue,> {
	let TreeValue::Scalar((value, line,),) = mir_value else { todo!() };
	Ok(match schema_value {
		Value::Single(single,) => TreeValue::Scalar(Value::Single(
			single.into_payload(key, &value, line,)?,
		),),
		Value::Collection(items,) => TreeValue::Scalar(Value::Collection(
			items
				.iter()
				.map(|single| single.into_payload(key, &value, line,),)
				.try_collect()?,
		),),
	},)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::parser::schema::SchemaValue;

	fn mir_scalar(value: &str, line: usize,) -> TreeValue<(String, usize,),> {
		TreeValue::Scalar((value.to_string(), line,),)
	}

	fn schema_scalar(kind: SingleValueDiscriminants,) -> SchemaValue {
		TreeValue::Scalar(Value::Single(kind,),)
	}

	#[test]
	fn parse_str_as_i32_parses_valid_integer() -> PRslt<(),> {
		assert_eq!(parse_str_as_i32("port", "42", 6)?, 42);
		Ok((),)
	}

	#[test]
	fn parse_str_as_i32_reports_invalid_value() -> PRslt<(),> {
		let err = parse_str_as_i32("port", "not-a-number", 3,).unwrap_err();
		match err {
			ParseError::InvalidValue { key, value, ty, line, } => {
				assert_eq!(key, "port");
				assert_eq!(value, "not-a-number");
				assert_eq!(ty, SingleValueDiscriminants::Integer);
				assert_eq!(line, 3);
			},
			other => panic!("unexpected error: {other:?}"),
		}

		Ok((),)
	}

	#[test]
	fn discriminant_into_payload_converts_bool() -> PRslt<(),> {
		let payload =
			SingleValueDiscriminants::Bool.into_payload("debug", "true", 5,)?;
		match payload {
			SingleValue::Bool(flag,) => assert!(flag),
			other => panic!("unexpected payload: {other:?}"),
		}

		Ok((),)
	}

	#[test]
	fn inject_payload_handles_single_value() -> PRslt<(),> {
		let schema_value = Value::Single(SingleValueDiscriminants::String,);
		let conf_value = inject_payload(
			"endpoint",
			&schema_value,
			mir_scalar("localhost", 4,),
		)?;
		match conf_value {
			TreeValue::Scalar(Value::Single(SingleValue::String(value,),),) => {
				assert_eq!(value, "localhost");
			},
			other => panic!("unexpected conf value: {other:?}"),
		}

		Ok((),)
	}

	#[test]
	fn inject_payload_handles_collection() -> PRslt<(),> {
		let schema_value = Value::Collection(vec![
			SingleValueDiscriminants::Integer,
			SingleValueDiscriminants::Integer,
		],);
		let conf_value =
			inject_payload("ports", &schema_value, mir_scalar("8080", 9,),)?;
		match conf_value {
			TreeValue::Scalar(Value::Collection(items,),) => {
				assert_eq!(items.len(), 2);
				assert!(
					items
						.iter()
						.all(|item| matches!(item, SingleValue::Integer(8080)))
				);
			},
			other => panic!("unexpected conf value: {other:?}"),
		}

		Ok((),)
	}

	#[test]
	fn structured_input_into_conf_converts_known_keys() -> PRslt<(),> {
		let mut mir = StructuredInput::new();
		mir.insert("debug".into(), mir_scalar("true", 1,),);
		mir.insert("port".into(), mir_scalar("21", 2,),);

		let mut schema = SchemaMap::new();
		schema.insert(
			"debug".into(),
			schema_scalar(SingleValueDiscriminants::Bool,),
		);
		schema.insert(
			"port".into(),
			schema_scalar(SingleValueDiscriminants::Integer,),
		);

		let conf = mir.into_conf(&schema,)?;

		match conf.get("debug",).unwrap() {
			TreeValue::Scalar(Value::Single(SingleValue::Bool(flag,),),) => {
				assert!(flag)
			},
			other => panic!("unexpected debug value: {other:?}"),
		}

		match conf.get("port",).unwrap() {
			TreeValue::Scalar(Value::Single(SingleValue::Integer(value,),),) =>
			{
				assert_eq!(*value, 21);
			},
			other => panic!("unexpected port value: {other:?}"),
		}

		Ok((),)
	}

	#[test]
	fn structured_input_into_conf_flags_unknown_keys() -> PRslt<(),> {
		let mut mir = StructuredInput::new();
		mir.insert("unexpected".into(), mir_scalar("true", 3,),);

		let schema = SchemaMap::new();
		let err = mir.into_conf(&schema,).unwrap_err();
		match err {
			ParseError::UnknownKey { key, lines, } => {
				assert_eq!(key, "unexpected");
				assert_eq!(lines, vec![3]);
			},
			other => panic!("unexpected error: {other:?}"),
		}

		Ok((),)
	}

	#[test]
	fn parse_str_resolves_nested_schema() -> PRslt<(),> {
		let mut nested_schema = SchemaMap::new();
		nested_schema.insert(
			"port".into(),
			schema_scalar(SingleValueDiscriminants::Integer,),
		);

		let schema = SchemaMap::from([(
			"server".to_string(),
			TreeValue::Map(nested_schema.into_inner(),),
		),],);

		let conf = parse_str("server.port = 8080", schema,)?;
		let server = conf.get("server",).unwrap();
		match server {
			TreeValue::Map(children,) => match children.get("port",).unwrap() {
				TreeValue::Scalar(Value::Single(SingleValue::Integer(
					value,
				),),) => {
					assert_eq!(*value, 8080);
				},
				other => panic!("unexpected port value: {other:?}"),
			},
			other => panic!("unexpected server value: {other:?}"),
		}

		Ok((),)
	}

	#[test]
	fn parse_str_propagates_unknown_key_error() -> PRslt<(),> {
		let schema = SchemaMap::new();
		let err = parse_str("feature.enabled = true", schema,).unwrap_err();
		match err {
			ParseError::UnknownKey { key, lines, } => {
				assert_eq!(key, "feature.enabled");
				assert_eq!(lines, vec![1]);
			},
			other => panic!("unexpected error: {other:?}"),
		}

		Ok((),)
	}
}
