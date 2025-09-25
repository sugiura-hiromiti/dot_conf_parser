use dot_conf_parser::error::ParseError;
use dot_conf_parser::parser::conf::SingleValueDiscriminants;
use dot_conf_parser::parser::conf::ValueDiscriminants;
use dot_conf_parser::parser::conf::{self};
use dot_conf_parser::parser::schema;
use std::error::Error as _;
use std::path::PathBuf;

#[test]
fn parse_error_display_covers_all_variants() {
	let io_err =
		ParseError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io",),);
	assert!(format!("{io_err}").starts_with("I/O error"));

	let missing = ParseError::MissingDelimiter { line: 7, };
	assert_eq!(format!("{missing}"), "missing delimiter on line 7");

	let empty_key = ParseError::EmptyKey { line: 3, };
	assert_eq!(format!("{empty_key}"), "empty key on line 3");

	let empty_value = ParseError::EmptyValue { line: 4, };
	assert_eq!(format!("{empty_value}"), "empty value on line 4");

	let invalid_segment =
		ParseError::InvalidKeySegment { segment: "".to_string(), line: 9, };
	assert_eq!(
		format!("{invalid_segment}"),
		"invalid key segment '' on line 9"
	);

	let conflict = ParseError::ConflictingTypes {
		key:  "server.port".to_string(),
		line: 11,
	};
	assert_eq!(
		format!("{conflict}"),
		"conflicting definitions for 'server.port' on line 11"
	);

	let invalid_value = ParseError::InvalidValue {
		key:   "service.mode".to_string(),
		value: "maybe".to_string(),
		ty:    SingleValueDiscriminants::Bool,
		line:  5,
	};
	let invalid_string = format!("{invalid_value}");
	assert!(invalid_string.contains("invalid value 'maybe'"));
	assert!(invalid_string.contains("Bool"));
	assert!(invalid_string.contains("service.mode"));

	let unknown = ParseError::UnknownKey {
		key:   "unknown".to_string(),
		lines: vec![2, 4],
	};
	assert_eq!(format!("{unknown}"), "unknown key 'unknown' on line [2, 4]");
}

#[test]
fn parse_error_source_only_wraps_io() {
	let io_err =
		ParseError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io",),);
	let source = io_err.source().expect("io source",);
	assert_eq!(source.to_string(), "io");

	let missing = ParseError::MissingDelimiter { line: 1, };
	assert!(missing.source().is_none());
}

#[test]
fn single_value_discriminants_display_all_variants() {
	assert_eq!(SingleValueDiscriminants::String.to_string(), "String");
	assert_eq!(SingleValueDiscriminants::Bool.to_string(), "Bool");
	assert_eq!(SingleValueDiscriminants::Integer.to_string(), "Integer");
}

#[test]
fn value_discriminants_display_variants() {
	assert_eq!(ValueDiscriminants::Single.to_string(), "Single");
	assert_eq!(ValueDiscriminants::Collection.to_string(), "Collection");
}

#[test]
fn parse_file_reports_io_errors() {
	let missing_path = {
		let mut path = std::env::temp_dir();
		path.push(format!(
			"missing_conf_{}_{}.conf",
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.expect("time",)
				.as_nanos(),
			std::process::id()
		),);
		path
	};

	let err = conf::parse_file(&missing_path, &missing_path,)
		.expect_err("conf parse should surface IO errors",);
	assert!(matches!(err, ParseError::Io(_)));

	let schema_err = schema::parse_file(PathBuf::from(missing_path,),)
		.expect_err("schema parse should surface IO errors",);
	assert!(matches!(schema_err, ParseError::Io(_)));
}
