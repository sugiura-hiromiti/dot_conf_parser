use crate::error::ParseError;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub mod error;
pub mod show;

pub type ConfMap = BTreeMap<String, ConfValue,>;

#[derive(Debug, Clone, PartialEq, Eq,)]
pub enum ConfValue {
	//  TODO: add value type like boolean, array, number ...
	Scalar(String,),
	Map(ConfMap,),
}

pub fn parse_file<P: AsRef<Path,>,>(path: P,) -> Result<ConfMap, ParseError,> {
	let mut file = File::open(path,)?;
	let mut contents = String::new();
	file.read_to_string(&mut contents,)?;
	parse_str(&contents,)
}

pub fn parse_str(input: &str,) -> Result<ConfMap, ParseError,> {
	let mut root = ConfMap::new();

	for (idx, raw_line,) in input.lines().enumerate() {
		let line_no = idx + 1;
		let trimmed = raw_line.trim();

		if trimmed.is_empty() {
			continue;
		}

		// we can assume that this `unwrap` do not panic, because it is ensured
		// `trimmed` is not empty
		let first_char = trimmed.chars().next().unwrap();
		if first_char == '#' || first_char == ';' {
			continue;
		}

		let (key_part, value_part,) = match raw_line.find('=',) {
			Some(eq_index,) => (
				raw_line[..eq_index].trim_end(),
				//  NOTE: this code is actually valid. see
				// `confirm_range_exp_valid_bound` test function
				raw_line[eq_index + 1..].trim_start(),
			),
			None => {
				return Err(ParseError::MissingDelimiter { line: line_no, },);
			},
		};

		let segments = parse_key(key_part, line_no,)?;

		let value = parse_value(value_part, line_no,)?;
		insert_value(&mut root, &segments, value, line_no,)?;
	}

	Ok(root,)
}

fn parse_key(
	key_part: &str,
	line_no: usize,
) -> Result<Vec<String,>, ParseError,> {
	if key_part.trim().is_empty() {
		return Err(ParseError::EmptyKey { line: line_no, },);
	}

	let segments: Vec<String,> = key_part
		.trim()
		.split('.',)
		.map(|segment| segment.trim(),)
		.map(|segment| segment.to_string(),)
		.collect();

	if segments.iter().any(|segment| segment.is_empty(),) {
		let bad = segments
			.into_iter()
			.find(|segment| segment.is_empty(),)
			.unwrap_or_default();
		return Err(ParseError::InvalidKeySegment {
			segment: bad,
			line:    line_no,
		},);
	}

	Ok(segments,)
}

fn parse_value(
	value_part: &str,
	line_no: usize,
) -> Result<String, ParseError,> {
	let without_comment = strip_inline_comment(value_part,);
	let trimmed = without_comment.trim();

	if trimmed.is_empty() {
		return Err(ParseError::EmptyValue { line: line_no, },);
	}

	Ok(trimmed.to_string(),)
}

fn strip_inline_comment(input: &str,) -> String {
	match input.find(['#', ';',],) {
		Some(cmt_index,) => input[..cmt_index].to_string(),
		None => input.to_string(),
	}
}

fn insert_value(
	root: &mut ConfMap,
	segments: &[String],
	value: String,
	line_no: usize,
) -> Result<(), ParseError,> {
	let mut current = root;
	for (idx, segment,) in segments.iter().enumerate() {
		let is_last = idx == segments.len() - 1;
		if is_last {
			match current.entry(segment.clone(),) {
				Entry::Vacant(entry,) => {
					entry.insert(ConfValue::Scalar(value.clone(),),);
				},
				Entry::Occupied(mut entry,) => match entry.get_mut() {
					ConfValue::Scalar(existing,) => {
						*existing = value.clone();
					},
					ConfValue::Map(_,) => {
						return Err(ParseError::ConflictingTypes {
							key:  segments[..=idx].join(".",),
							line: line_no,
						},);
					},
				},
			}
		} else {
			// do noting for segment engties already exist
			if let Entry::Vacant(entry,) = current.entry(segment.clone(),) {
				// NOTE: entry should be map because current segment is not at
				// last
				entry.insert(ConfValue::Map(ConfMap::new(),),);
			}

			current = match current.get_mut(segment,) {
				Some(ConfValue::Map(map,),) => map,
				//  NOTE: reject nested assignment
				//  (like a.b.c.d = xxx with a.b.c = yyy)
				Some(ConfValue::Scalar(_,),) => {
					return Err(ParseError::ConflictingTypes {
						key:  segments[..=idx].join(".",),
						line: line_no,
					},);
				},
				None => unreachable!(),
			};
		}
	}

	Ok((),)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn confirm_range_exp_valid_bound() {
		let a = "abc =";
		let key = &a[..4];
		let val = &a[5..];
		assert_eq!(key, "abc ");
		assert_eq!(val, "");
	}

	#[test]
	#[should_panic]
	fn confirm_range_exp_invalid_bound() {
		let a = "abc =";
		let _key = &a[..4];
		let _val = &a[6..];
		// assert_eq!(key, "abc ");
		// assert_eq!(val, "");
	}

	#[test]
	fn parses_basic_entries() -> Result<(), ParseError,> {
		let input = "endpoint = localhost:3000\ndebug = true\nlog.file = \
		             /var/log/console.log\n";
		let parsed = parse_str(input,)?;

		assert_eq!(
			parsed.get("endpoint"),
			Some(&ConfValue::Scalar("localhost:3000".to_string()))
		);
		assert_eq!(
			parsed.get("debug"),
			Some(&ConfValue::Scalar("true".to_string()))
		);

		let log = parsed.get("log",).expect("missing log entry",);
		match log {
			ConfValue::Map(map,) => {
				assert_eq!(
					map.get("file"),
					Some(&ConfValue::Scalar(
						"/var/log/console.log".to_string()
					))
				);
			},
			_ => panic!("expected log to be a map"),
		}

		Ok((),)
	}

	#[test]
	fn skips_comments_and_overrides() -> Result<(), ParseError,> {
		let input = "endpoint = localhost:3000\n# debug = true\nlog.file = \
		             /var/log/console.log\nlog.name = default.log\n";
		let parsed = parse_str(input,)?;

		assert_eq!(
			parsed.get("endpoint"),
			Some(&ConfValue::Scalar("localhost:3000".to_string()))
		);
		assert!(parsed.get("debug").is_none());

		let log = parsed.get("log",).expect("missing log entry",);
		match log {
			ConfValue::Map(map,) => {
				assert_eq!(
					map.get("file"),
					Some(&ConfValue::Scalar(
						"/var/log/console.log".to_string()
					))
				);
				assert_eq!(
					map.get("name"),
					Some(&ConfValue::Scalar("default.log".to_string()))
				);
			},
			_ => panic!("expected log to be a map"),
		}

		Ok((),)
	}
}
