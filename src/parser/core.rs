use crate::error::PRslt;
use crate::error::ParseError;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fmt::Debug;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq,)]
pub enum TreeValue<T,> {
	Scalar(T,),
	Map(BTreeMap<String, TreeValue<T,>,>,),
}

impl TreeValue<(String, usize,),> {
	pub fn get_lines_of_key(&self,) -> Vec<usize,> {
		match self {
			Self::Scalar((_, l,),) => vec![*l],
			Self::Map(btree_map,) => btree_map
				.iter()
				.flat_map(|(_, v,)| v.get_lines_of_key(),)
				.collect(),
		}
	}
}

pub trait Valuable {
	fn sep() -> &'static str;

	fn extract_key_value(s: &str, line_no: usize,) -> PRslt<(&str, &str,),> {
		let sep = Self::sep();
		match s.find(sep,) {
			Some(eq_index,) => {
				let key_part = &s[..eq_index];
				let value_part = &s[eq_index + sep.len()..];
				Ok((
					key_part.trim(),
					//  NOTE: this code is actually valid. see
					// `confirm_range_exp_valid_bound` test function
					value_part,
				),)
			},
			None => Err(ParseError::MissingDelimiter { line: line_no, },),
		}
	}
}

/// mir
pub type StructuredInput = BTreeMap<String, TreeValue<(String, usize,),>,>;

pub(crate) fn file_to_mir<P: AsRef<Path,>, V: Valuable,>(
	path: P,
	// line_parser: impl Fn(&str,) -> Result<(&str, &str,),>,
) -> PRslt<StructuredInput,> {
	let mut file = File::open(path,)?;
	let mut contents = String::new();
	file.read_to_string(&mut contents,)?;
	str_to_mir::<V,>(&contents,)
}

pub(crate) fn str_to_mir<V: Valuable,>(
	input: &str,
) -> PRslt<StructuredInput,> {
	let mut root = StructuredInput::new();

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

		let (key_part, value_part,) = V::extract_key_value(trimmed, line_no,)?;

		let segments = parse_key(key_part, line_no,)?;

		let value = parse_value(value_part, line_no,)?;
		insert_value(&mut root, &segments, value, line_no,)?;
	}

	Ok(root,)
}

fn parse_key(key_part: &str, line_no: usize,) -> PRslt<Vec<String,>,> {
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

fn parse_value(value_part: &str, line_no: usize,) -> PRslt<String,> {
	let without_comment = strip_inline_comment(value_part,);
	let trimmed = without_comment.trim();

	if trimmed.is_empty() {
		return Err(ParseError::EmptyValue { line: line_no, },);
	}

	let mut normalized = String::with_capacity(trimmed.len(),);
	let mut last_was_space = false;

	for ch in trimmed.chars() {
		if ch.is_whitespace() {
			if !normalized.is_empty() && !last_was_space {
				normalized.push(' ',);
				last_was_space = true;
			}
		} else {
			normalized.push(ch,);
			last_was_space = false;
		}
	}

	Ok(normalized,)
}

fn strip_inline_comment(input: &str,) -> String {
	match input.find(['#', ';',],) {
		Some(cmt_index,) => input[..cmt_index].to_string(),
		None => input.to_string(),
	}
}

fn insert_value(
	root: &mut StructuredInput,
	segments: &[String],
	value: String,
	line_no: usize,
) -> PRslt<(),> {
	let mut current = root;
	for (idx, segment,) in segments.iter().enumerate() {
		let is_last = idx == segments.len() - 1;
		if is_last {
			match current.entry(segment.clone(),) {
				Entry::Vacant(entry,) => {
					entry.insert(TreeValue::Scalar((
						value.to_string(),
						line_no,
					),),);
				},
				Entry::Occupied(mut entry,) => match entry.get_mut() {
					TreeValue::Scalar(existing,) => {
						existing.0 = value.to_string();
						existing.1 = line_no;
					},
					TreeValue::Map(_,) => {
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
				entry.insert(TreeValue::Map(StructuredInput::new(),),);
			}

			current = match current.get_mut(segment,) {
				Some(TreeValue::Map(map,),) => map,
				//  NOTE: reject nested assignment
				//  (like a.b.c.d = xxx with a.b.c = yyy)
				Some(TreeValue::Scalar(_,),) => {
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
	use crate::parser::conf::SingleValue;

	#[test]
	fn extract_key_value_uses_type_separator() {
		let (key, value,) =
			SingleValue::extract_key_value("alpha = beta", 3,).unwrap();
		assert_eq!(key, "alpha");
		assert_eq!(value, " beta");
	}

	#[test]
	fn extract_key_value_missing_separator_surfaces_error() {
		let err =
			SingleValue::extract_key_value("no_delimiter", 4,).unwrap_err();
		match err {
			ParseError::MissingDelimiter { line, } => assert_eq!(line, 4),
			other => panic!("unexpected error: {other:?}"),
		}
	}

	#[test]
	fn parse_key_rejects_empty_segments() {
		let err = parse_key("foo..bar", 8,).unwrap_err();
		match err {
			ParseError::InvalidKeySegment { segment, line, } => {
				assert_eq!(segment, "");
				assert_eq!(line, 8);
			},
			other => panic!("unexpected error: {other:?}"),
		}
	}

	#[test]
	fn parse_key_happy_path() {
		let key_segments = parse_key(" network . ipv4 . port", 1,).unwrap();
		assert_eq!(key_segments, vec!["network", "ipv4", "port"]);
	}

	#[test]
	fn parse_value_trims_and_ignores_inline_comment() {
		let value = parse_value(" on 	 value ; comment ", 5,).unwrap();
		assert_eq!(value, "on value");
	}

	#[test]
	fn parse_value_rejects_empty_payload() {
		let err = parse_value("   # fully commented", 2,).unwrap_err();
		match err {
			ParseError::EmptyValue { line, } => assert_eq!(line, 2),
			other => panic!("unexpected error: {other:?}"),
		}
	}

	#[test]
	fn str_to_mir_ignores_comments_and_blank_lines() {
		let input = "# heading\n\n endpoint = localhost \n log.file = \
		             /tmp/out.log # trailing";
		let mir = str_to_mir::<SingleValue,>(input,).unwrap();

		let endpoint = mir.get("endpoint",).unwrap();
		assert_eq!(endpoint, &TreeValue::Scalar(("localhost".to_string(), 3)));

		let nested = mir.get("log",).unwrap();
		match nested {
			TreeValue::Map(children,) => {
				let value = children.get("file",).unwrap();
				assert_eq!(
					value,
					&TreeValue::Scalar(("/tmp/out.log".to_string(), 4))
				);
			},
			other => panic!("expected map, got {other:?}"),
		}
	}

	#[test]
	fn str_to_mir_rejects_conflicting_types() {
		let input = "foo = one\nfoo.bar = two";
		let err = str_to_mir::<SingleValue,>(input,).unwrap_err();
		match err {
			ParseError::ConflictingTypes { key, line, } => {
				assert_eq!(key, "foo");
				assert_eq!(line, 2);
			},
			other => panic!("unexpected error: {other:?}"),
		}
	}

	#[test]
	fn tree_value_reports_all_line_numbers() {
		let tree = TreeValue::Map(BTreeMap::from([
			(
				"first".to_string(),
				TreeValue::Scalar(("value".to_string(), 7,),),
			),
			(
				"nested".to_string(),
				TreeValue::Map(BTreeMap::from([(
					"inner".to_string(),
					TreeValue::Scalar(("another".to_string(), 11,),),
				),],),),
			),
		],),);

		let mut lines = tree.get_lines_of_key();
		lines.sort();
		assert_eq!(lines, vec![7, 11]);
	}
}
