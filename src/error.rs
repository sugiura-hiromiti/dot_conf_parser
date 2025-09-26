use crate::parser::conf::SingleValueDiscriminants;

#[derive(Debug,)]
pub enum ParseError {
	Io(std::io::Error,),
	/// missing `=`  or `->`
	MissingDelimiter {
		line: usize,
	},
	EmptyKey {
		line: usize,
	},
	EmptyValue {
		line: usize,
	},
	InvalidKeySegment {
		segment: String,
		line:    usize,
	},
	/// case of declarating a certain key multiple times
	ConflictingTypes {
		key:  String,
		line: usize,
	},
	InvalidValue {
		key:   String,
		value: String,
		ty:    SingleValueDiscriminants,
		line:  usize,
	},
	UnknownKey {
		key:   String,
		lines: Vec<usize,>,
	},
}

impl std::fmt::Display for ParseError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result {
		match self {
			ParseError::Io(err,) => write!(f, "I/O error: {err}"),
			ParseError::MissingDelimiter { line, } => {
				write!(f, "missing delimiter on line {line}")
			},
			ParseError::EmptyKey { line, } => {
				write!(f, "empty key on line {line}")
			},
			ParseError::EmptyValue { line, } => {
				write!(f, "empty value on line {line}")
			},
			ParseError::InvalidKeySegment { segment, line, } => {
				write!(f, "invalid key segment '{segment}' on line {line}")
			},
			ParseError::ConflictingTypes { key, line, } => {
				write!(f, "conflicting definitions for '{key}' on line {line}")
			},
			ParseError::InvalidValue { key, value, ty, line, } => {
				write!(
					f,
					"invalid value '{value}' while expecting {ty} for '{key}' \
					 on line {line}"
				)
			},
			ParseError::UnknownKey { key, lines, } => {
				write!(f, "unknown key '{key}' on line {lines:?}")
			},
		}
	}
}

impl std::error::Error for ParseError {
	fn source(&self,) -> Option<&(dyn std::error::Error + 'static),> {
		match self {
			ParseError::Io(err,) => Some(err,),
			_ => None,
		}
	}
}

impl From<std::io::Error,> for ParseError {
	fn from(value: std::io::Error,) -> Self {
		ParseError::Io(value,)
	}
}

impl From<strum::ParseError,> for ParseError {
	fn from(_: strum::ParseError,) -> Self {
		Self::InvalidValue {
			key:   "".to_string(),
			value: "".to_string(),
			ty:    SingleValueDiscriminants::Bool,
			line:  0,
		}
	}
}

pub type PRslt<T,> = Result<T, ParseError,>;

#[cfg(test)]
mod tests {
	use super::*;
	use std::io;
	use std::str::FromStr;

	#[test]
	fn display_formats_missing_delimiter() {
		let msg = ParseError::MissingDelimiter { line: 12, }.to_string();
		assert_eq!(msg, "missing delimiter on line 12");
	}

	#[test]
	fn display_formats_invalid_value_payload() {
		let err = ParseError::InvalidValue {
			key:   "flag".to_string(),
			value: "yes".to_string(),
			ty:    SingleValueDiscriminants::Bool,
			line:  7,
		};
		let msg = err.to_string();
		assert_eq!(
			msg,
			"invalid value 'yes' while expecting Bool for 'flag' on line 7",
		);
	}

	#[test]
	fn io_error_conversion_wraps_source() {
		let io_err = io::Error::new(io::ErrorKind::Other, "boom",);
		let parse_err: ParseError = io_err.into();
		match parse_err {
			ParseError::Io(inner,) => {
				assert_eq!(inner.kind(), io::ErrorKind::Other)
			},
			other => panic!("unexpected error: {other:?}"),
		}
	}

	#[test]
	fn strum_error_conversion_defaults_to_invalid_value() {
		let parse_err =
			SingleValueDiscriminants::from_str("unsupported",).unwrap_err();
		let converted: ParseError = parse_err.into();
		match converted {
			ParseError::InvalidValue { key, value, ty, line, } => {
				assert!(key.is_empty());
				assert!(value.is_empty());
				assert_eq!(ty, SingleValueDiscriminants::Bool);
				assert_eq!(line, 0);
			},
			other => panic!("unexpected error: {other:?}"),
		}
	}

	#[test]
	fn display_lists_unknown_key_lines() {
		let err = ParseError::UnknownKey {
			key:   "db.port".to_string(),
			lines: vec![5, 9],
		};
		let msg = err.to_string();
		assert!(msg.contains("db.port"));
		assert!(msg.contains("5"));
		assert!(msg.contains("9"));
	}
}
