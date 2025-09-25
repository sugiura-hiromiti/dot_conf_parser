use dot_conf_parser::error::ParseError;
use dot_conf_parser::parser::conf::SingleValueDiscriminants;
use dot_conf_parser::parser::conf::Value;
use dot_conf_parser::parser::schema::SchemaValue;
use dot_conf_parser::parser::schema::{self};

#[test]
fn schema_parses_collection_values() {
	let schema =
		schema::parse_str("limits -> Integer, Bool\n",).expect("schema parse",);

	match schema.get("limits",).expect("missing limits entry",) {
		SchemaValue::Scalar(Value::Collection(items,),) => {
			assert_eq!(
				items,
				&vec![
					SingleValueDiscriminants::Integer,
					SingleValueDiscriminants::Bool,
				]
			);
		},
		other => panic!("expected collection schema, got {other:?}"),
	}
}

#[test]
fn schema_strips_inline_comments() {
	let schema = schema::parse_str(
		"log.level -> String # we support arbitrary strings\n",
	)
	.expect("schema parse",);

	match schema.get("log.level",).expect("missing log.level entry",) {
		SchemaValue::Scalar(Value::Single(kind,),) => {
			assert_eq!(kind, &SingleValueDiscriminants::String);
		},
		other => panic!("expected single value schema, got {other:?}"),
	}
}

#[test]
fn schema_reports_missing_delimiter() {
	let err = schema::parse_str("log.level String\n",)
		.expect_err("expected delimiter error",);

	match err {
		ParseError::MissingDelimiter { line, } => assert_eq!(line, 1),
		other => panic!("unexpected error: {other}"),
	}
}

#[test]
fn schema_reports_empty_key() {
	let err = schema::parse_str("   -> Bool\n",)
		.expect_err("expected empty key error",);

	match err {
		ParseError::EmptyKey { line, } => assert_eq!(line, 1),
		other => panic!("unexpected error: {other}"),
	}
}

#[test]
fn schema_reports_empty_value() {
	let err = schema::parse_str("flag ->   # comment only\n",)
		.expect_err("expected empty value error",);

	match err {
		ParseError::EmptyValue { line, } => assert_eq!(line, 1),
		other => panic!("unexpected error: {other}"),
	}
}

#[test]
fn schema_reports_invalid_segment() {
	let err = schema::parse_str("log..level -> Bool\n",)
		.expect_err("expected invalid segment error",);

	match err {
		ParseError::InvalidKeySegment { segment, line, } => {
			assert!(segment.is_empty());
			assert_eq!(line, 1);
		},
		other => panic!("unexpected error: {other}"),
	}
}

#[test]
fn schema_supports_nested_maps() {
	let schema =
		schema::parse_str("service.mode -> String\n",).expect("schema parse",);

	match schema.get("service",).expect("missing root service entry",) {
		SchemaValue::Map(map,) => {
			assert!(map.contains_key("mode"));
		},
		other => panic!("expected schema map, got {other:?}"),
	}
}

#[test]
fn schema_ignores_indented_comments() {
	let schema_src = "   # leading comment\nservice.mode -> String\n    ; \
	                  trailing comment\n";
	let schema = schema::parse_str(schema_src,).expect("schema parse",);

	match schema.get("service",).expect("missing service entry",) {
		SchemaValue::Map(map,) => assert!(map.contains_key("mode")),
		other => panic!("expected schema map, got {other:?}"),
	}
}

#[test]
fn schema_parses_from_file() {
	let mut path = std::env::temp_dir();
	let unique = format!(
		"schema_test_{}.conf",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.expect("time")
			.as_nanos()
	);
	path.push(unique,);
	std::fs::write(&path, "service.mode -> String\n",).expect("write schema",);

	let parsed = schema::parse_file(&path,).expect("schema parse",);
	match parsed.get("service",).expect("service entry",) {
		SchemaValue::Map(map,) => {
			assert!(map.contains_key("mode"));
		},
		other => panic!("expected nested map, got {other:?}"),
	}
	std::fs::remove_file(path,).expect("cleanup",);
}

#[test]
fn schema_rejects_unknown_value_type() {
	let err = schema::parse_str("feature.flag -> Unknown\n",)
		.expect_err("expected invalid value error",);

	match err {
		ParseError::InvalidValue { ty, .. } => {
			assert_eq!(ty.to_string(), "Bool");
		},
		other => panic!("unexpected error: {other}"),
	}
}
