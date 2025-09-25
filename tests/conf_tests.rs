use dot_conf_parser::error::PRslt;
use dot_conf_parser::error::ParseError;
use dot_conf_parser::parser::conf::ConfValue;
use dot_conf_parser::parser::conf::SingleValue;
use dot_conf_parser::parser::conf::Value;
use dot_conf_parser::parser::conf::{self};
use dot_conf_parser::parser::schema;
use proptest::prelude::*;

fn expect_string(value: &ConfValue,) -> &str {
	match value {
		ConfValue::Scalar(Value::Single(SingleValue::String(s,),),) => s,
		other => panic!("expected string payload, got {other:?}"),
	}
}

fn expect_bool(value: &ConfValue,) -> bool {
	match value {
		ConfValue::Scalar(Value::Single(SingleValue::Bool(flag,),),) => *flag,
		other => panic!("expected bool payload, got {other:?}"),
	}
}

fn expect_int(value: &ConfValue,) -> i32 {
	match value {
		ConfValue::Scalar(Value::Single(SingleValue::Integer(v,),),) => *v,
		other => panic!("expected integer payload, got {other:?}"),
	}
}

fn expect_ints(value: &ConfValue,) -> Vec<i32,> {
	match value {
		ConfValue::Scalar(Value::Collection(items,),) => items
			.iter()
			.map(|entry| match entry {
				SingleValue::Integer(v,) => *v,
				other => panic!("expected integer, got {other:?}"),
			},)
			.collect(),
		other => panic!("expected collection, got {other:?}"),
	}
}

#[test]
fn conf_overwrites_duplicate_scalar_values() -> PRslt<(),> {
	let schema = schema::parse_str("name -> String\n",)?;
	let conf = conf::parse_str("name = original\nname = updated\n", schema,)?;

	assert_eq!(expect_string(conf.get("name").expect("name entry")), "updated");
	Ok((),)
}

#[test]
fn conf_supports_inline_comments() -> PRslt<(),> {
	let schema = schema::parse_str("net.port -> Integer\n",)?;
	let conf = conf::parse_str("net.port = 443 # https\n", schema,)?;

	assert_eq!(expect_int(conf.get("net.port").expect("net.port entry")), 443);
	Ok((),)
}

#[test]
fn conf_trims_key_segments() -> PRslt<(),> {
	let schema = schema::parse_str("outer.inner -> String\n",)?;
	let conf = conf::parse_str("outer . inner = spaced\n", schema,)?;

	assert_eq!(
		expect_string(
			conf.get("outer")
				.and_then(|m| match m {
					ConfValue::Map(map,) => map.get("inner"),
					other => panic!("expected nested map, got {other:?}"),
				})
				.expect("inner entry")
		),
		"spaced"
	);

	Ok((),)
}

#[test]
fn conf_reports_empty_value_after_comment() -> PRslt<(),> {
	let schema = schema::parse_str("service.enabled -> Bool\n",)?;
	let err = conf::parse_str("service.enabled =   ; no value\n", schema,)
		.expect_err("expected empty value error",);

	match err {
		ParseError::EmptyValue { line, } => assert_eq!(line, 1),
		other => panic!("unexpected error: {other}"),
	}

	Ok((),)
}

#[test]
fn conf_reports_unknown_keys_with_all_lines() -> PRslt<(),> {
	let schema = schema::parse_str("service.mode -> String\n",)?;
	let err = conf::parse_str(
		"service.mode = maintenance\nunknown.flag = true\n",
		schema,
	)
	.expect_err("expected unknown key error",);

	match err {
		ParseError::UnknownKey { key, lines, } => {
			assert_eq!(key, "unknown");
			assert_eq!(lines, vec![2]);
		},
		other => panic!("unexpected error: {other}"),
	}

	Ok((),)
}

#[test]
fn conf_builds_collections_from_schema() -> PRslt<(),> {
	let schema = schema::parse_str("limits -> Integer, Integer\n",)?;
	let conf = conf::parse_str("limits = 7\n", schema,)?;

	assert_eq!(
		expect_ints(conf.get("limits").expect("limits entry")),
		vec![7, 7]
	);

	Ok((),)
}

#[test]
fn conf_rejects_invalid_integer_values() -> PRslt<(),> {
	let schema = schema::parse_str("retry.count -> Integer\n",)?;
	let err = conf::parse_str("retry.count = not-a-number\n", schema,)
		.expect_err("expected invalid value error",);

	match err {
		ParseError::InvalidValue { key, value, ty, line, } => {
			assert_eq!(key, "retry.count");
			assert_eq!(value, "not-a-number");
			assert_eq!(ty.to_string(), "Integer");
			assert_eq!(line, 1);
		},
		other => panic!("unexpected error: {other}"),
	}

	Ok((),)
}

#[test]
fn conf_reports_missing_delimiter() -> PRslt<(),> {
	let schema = schema::parse_str("name -> String\n",)?;
	let err = conf::parse_str("name value without equals\n", schema,)
		.expect_err("expected missing delimiter error",);

	match err {
		ParseError::MissingDelimiter { line, } => assert_eq!(line, 1),
		other => panic!("unexpected error: {other}"),
	}

	Ok((),)
}

#[test]
fn conf_reports_unknown_nested_key_with_all_lines() -> PRslt<(),> {
	let schema = schema::parse_str("service.mode -> String\n",)?;
	let err = conf::parse_str(
		"service.mode = production\nunknown.flag = true\nunknown.level = \
		 critical\n",
		schema,
	)
	.expect_err("expected unknown key error",);

	match err {
		ParseError::UnknownKey { key, lines, } => {
			assert_eq!(key, "unknown");
			assert_eq!(lines, vec![2, 3]);
		},
		other => panic!("unexpected error: {other}"),
	}

	Ok((),)
}

#[test]
fn conf_reports_latest_line_for_overwritten_unknown_leaf() -> PRslt<(),> {
	let schema = schema::parse_str("service.mode -> String\n",)?;
	let err =
		conf::parse_str("unknown.flag = true\nunknown.flag = false\n", schema,)
			.expect_err("expected unknown key error",);

	match err {
		ParseError::UnknownKey { key, lines, } => {
			assert_eq!(key, "unknown");
			assert_eq!(lines, vec![2]);
		},
		other => panic!("unexpected error: {other}"),
	}

	Ok((),)
}

#[test]
fn conf_rejects_scalar_after_nested_map() -> PRslt<(),> {
	let schema = schema::parse_str("service.mode -> String\n",)?;
	let err = conf::parse_str(
		"service.mode = production\nservice = basic\n",
		schema,
	)
	.expect_err("expected conflicting type error",);

	match err {
		ParseError::ConflictingTypes { key, line, } => {
			assert_eq!(key, "service");
			assert_eq!(line, 2);
		},
		other => panic!("unexpected error: {other}"),
	}

	Ok((),)
}

#[test]
fn conf_rejects_nested_assignment_after_scalar() -> PRslt<(),> {
	let schema = schema::parse_str("service.mode -> String\n",)?;
	let err =
		conf::parse_str("service = basic\nservice.mode = advanced\n", schema,)
			.expect_err("expected conflicting type error",);

	match err {
		ParseError::ConflictingTypes { key, line, } => {
			assert_eq!(key, "service");
			assert_eq!(line, 2);
		},
		other => panic!("unexpected error: {other}"),
	}

	Ok((),)
}

#[test]
fn conf_supports_semicolon_inline_comments() -> PRslt<(),> {
	let schema = schema::parse_str("path -> String\n",)?;
	let conf = conf::parse_str("path = /tmp/data ; keep last\n", schema,)
		.expect("conf parse",);

	assert_eq!(
		expect_string(conf.get("path").expect("path entry")),
		"/tmp/data"
	);

	Ok((),)
}

#[test]
fn conf_supports_negative_integers() -> PRslt<(),> {
	let schema = schema::parse_str("retry.count -> Integer\n",)?;
	let conf = conf::parse_str("retry.count = -42\n", schema,)?;

	assert_eq!(
		expect_int(conf.get("retry.count").expect("retry.count entry")),
		-42
	);

	Ok((),)
}

#[test]
fn conf_trims_trailing_whitespace_in_values() -> PRslt<(),> {
	let schema = schema::parse_str("path -> String\n",)?;
	let conf = conf::parse_str("path = /var/log/app   \n", schema,)?;

	assert_eq!(
		expect_string(conf.get("path").expect("path entry")),
		"/var/log/app"
	);

	Ok((),)
}

#[test]
fn conf_ignores_blank_and_comment_lines() -> PRslt<(),> {
	let schema = schema::parse_str("service.name -> String\n",)?;
	let conf_src =
		"\n# skipped comment\n; another comment\nservice.name = running\n";
	let conf = conf::parse_str(conf_src, schema,)?;

	assert_eq!(
		expect_string(conf.get("service.name").expect("service.name entry")),
		"running"
	);

	Ok((),)
}

#[test]
fn conf_rejects_empty_key() -> PRslt<(),> {
	let schema = schema::parse_str("service.name -> String\n",)?;
	let err = conf::parse_str(" = value\n", schema,)
		.expect_err("expected empty key error",);

	match err {
		ParseError::EmptyKey { line, } => assert_eq!(line, 1),
		other => panic!("unexpected error: {other}"),
	}

	Ok((),)
}

#[test]
fn conf_rejects_invalid_key_segment() -> PRslt<(),> {
	let schema = schema::parse_str("service.name -> String\n",)?;
	let err = conf::parse_str("service..name = value\n", schema,)
		.expect_err("expected invalid key segment",);

	match err {
		ParseError::InvalidKeySegment { segment, line, } => {
			assert!(segment.is_empty());
			assert_eq!(line, 1);
		},
		other => panic!("unexpected error: {other}"),
	}

	Ok((),)
}

#[test]
fn conf_parses_from_file() -> PRslt<(),> {
	let mut schema_path = std::env::temp_dir();
	let mut conf_path = std::env::temp_dir();
	let unique = format!(
		"conf_test_{}_{}",
		std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.expect("time")
			.as_nanos(),
		std::process::id()
	);
	schema_path.push(format!("{unique}_schema.conf"),);
	conf_path.push(format!("{unique}_conf.conf"),);
	std::fs::write(&schema_path, "app.port -> Integer\n",)?;
	std::fs::write(&conf_path, "app.port = 9000\n",)?;

	let conf_map = conf::parse_file(&conf_path, &schema_path,)?;
	assert_eq!(
		expect_int(conf_map.get("app.port").expect("app.port entry")),
		9000
	);

	std::fs::remove_file(&schema_path,)?;
	std::fs::remove_file(&conf_path,)?;

	Ok((),)
}

proptest! {
	#[test]
	fn bool_payload_matches_true_literal(input in prop::string::string_regex("[A-Za-z0-9]+").unwrap()) {
		prop_assume!(!input.is_empty());

		let schema = schema::parse_str("feature.enabled -> Bool\n").expect("schema parse");
		let conf_string = format!("feature.enabled = {}\n", input);
		let conf = conf::parse_str(&conf_string, schema).expect("conf parse");

		let value = expect_bool(conf.get("feature.enabled").expect("feature.enabled entry"));
		prop_assert_eq!(value, input == "true");
	}

	#[test]
	fn integer_payload_round_trips(input in any::<i32>()) {
		let schema = schema::parse_str("retry.count -> Integer\n").expect("schema parse");
		let conf_string = format!("retry.count = {}\n", input);
		let conf = conf::parse_str(&conf_string, schema).expect("conf parse");

		let value = expect_int(conf.get("retry.count").expect("retry.count entry"));
		prop_assert_eq!(value, input);
	}
}

#[test]
fn conf_parses_bool_true_literal() -> PRslt<(),> {
	let schema = schema::parse_str("feature.enabled -> Bool\n",)?;
	let conf = conf::parse_str("feature.enabled = true\n", schema,)?;

	assert!(expect_bool(
		conf.get("feature.enabled").expect("feature.enabled entry")
	));

	Ok((),)
}

#[test]
fn conf_parses_bool_false_literal() -> PRslt<(),> {
	let schema = schema::parse_str("feature.enabled -> Bool\n",)?;
	let conf = conf::parse_str("feature.enabled = false\n", schema,)?;

	assert!(!expect_bool(
		conf.get("feature.enabled").expect("feature.enabled entry"),
	));

	Ok((),)
}

#[test]
fn conf_supports_boolean_collections() -> PRslt<(),> {
	let schema = schema::parse_str("feature.flags -> Bool, Bool\n",)?;
	let conf = conf::parse_str("feature.flags = true\n", schema,)?;

	match conf.get("feature.flags",).expect("feature.flags entry",) {
		ConfValue::Scalar(Value::Collection(items,),) => {
			assert_eq!(items.len(), 2);
			for item in items {
				match item {
					SingleValue::Bool(flag,) => assert!(*flag),
					other => panic!("expected bool payload, got {other:?}"),
				}
			}
		},
		other => panic!("expected collection payload, got {other:?}"),
	}

	Ok((),)
}
