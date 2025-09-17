use dot_conf_parser::ConfValue;
use dot_conf_parser::error::ParseError;
use dot_conf_parser::parse_file;
use std::path::PathBuf;

fn example_path(name: &str,) -> PathBuf {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"),);
	path.push("tests/examples",);
	path.push(name,);
	path
}

fn as_map(
	value: &ConfValue,
) -> &std::collections::BTreeMap<String, ConfValue,> {
	match value {
		ConfValue::Map(map,) => map,
		other => panic!("expected map, got: {other:?}"),
	}
}

fn scalar(value: &ConfValue,) -> &str {
	match value {
		ConfValue::Scalar(s,) => s,
		other => panic!("expected scalar, got: {other:?}"),
	}
}

#[test]
fn parses_sysctl_sample() {
	let path = example_path("sysctl_sample.conf",);
	dbg!(&path);
	let parsed =
		parse_file(path,).expect("failed to parse sysctl_sample.conf",);

	let kernel =
		as_map(parsed.get("kernel",).expect("missing kernel section",),);
	assert_eq!(
		scalar(kernel.get("domainname").expect("domainname")),
		"example.com"
	);
	assert_eq!(scalar(kernel.get("hostname").expect("hostname")), "host-01");

	let service =
		as_map(parsed.get("service",).expect("missing service section",),);
	assert_eq!(scalar(service.get("mode").expect("mode")), "maintenance");

	assert_eq!(
		scalar(
			parsed
				.get("path")
				.and_then(|m| as_map(m).get("with"))
				.and_then(|m| as_map(m).get("space"))
				.expect("path.with.space"),
		),
		"/tmp/test\\ folder"
	);
}

#[test]
fn reports_nested_assignment_error() {
	let path = example_path("sysctl_nested_assignment.conf",);
	let err = parse_file(path,).expect_err("expected conflicting type error",);

	match err {
		ParseError::ConflictingTypes { key, line, } => {
			assert_eq!(key, "service");
			assert_eq!(line, 2);
		},
		other => panic!("unexpected error: {other}"),
	}
}
