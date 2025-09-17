#[derive(Debug,)]
pub enum ParseError {
	Io(std::io::Error,),
	/// missing `=`
	MissingDelimiter {
		line: usize,
	},
	EmptyKey {
		line: usize,
	},
	EmptyValue {
		line: usize,
	},
	//  TODO: Currently, the only possible value for segment is an empty
	// string (""). the variant name InvalidKeySegment remains semantically
	// valid when the value is injected by another program.
	InvalidKeySegment {
		segment: String,
		line:    usize,
	},
	/// declarating a certain key multiple times
	ConflictingTypes {
		key:  String,
		line: usize,
	},
	UnterminatedQuote {
		line: usize,
	},
	TrailingEscape {
		line: usize,
	},
}

impl std::fmt::Display for ParseError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_,>,) -> std::fmt::Result {
		match self {
			ParseError::Io(err,) => write!(f, "I/O error: {err}"),
			ParseError::MissingDelimiter { line, } => {
				write!(f, "missing '=' delimiter on line {line}")
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
			ParseError::UnterminatedQuote { line, } => {
				write!(f, "unterminated quoted value on line {line}")
			},
			ParseError::TrailingEscape { line, } => {
				write!(f, "trailing escape sequence on line {line}")
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
