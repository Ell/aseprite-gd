use std::fmt;

/// Structured parse error. Always carries the absolute byte offset so importer
/// diagnostics can say *where* a file is corrupt, not just that it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Ran off the end of the buffer.
    UnexpectedEof { offset: usize, needed: usize },
    /// A magic number didn't match (file header 0xA5E0, frame header 0xF1FA).
    BadMagic { offset: usize, expected: u16, found: u16 },
    /// A field held a value the spec does not allow.
    Invalid { offset: usize, what: &'static str },
    /// A string field was not valid UTF-8.
    BadString { offset: usize },
    /// A safety limit from [`crate::limits`] was exceeded (possible zip bomb
    /// or otherwise hostile file).
    LimitExceeded { offset: usize, what: &'static str },
}

impl ParseError {
    pub fn offset(&self) -> usize {
        match *self {
            ParseError::UnexpectedEof { offset, .. }
            | ParseError::BadMagic { offset, .. }
            | ParseError::Invalid { offset, .. }
            | ParseError::BadString { offset }
            | ParseError::LimitExceeded { offset, .. } => offset,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedEof { offset, needed } => {
                write!(f, "unexpected end of file at offset {offset} (needed {needed} more bytes)")
            }
            ParseError::BadMagic { offset, expected, found } => {
                write!(f, "bad magic at offset {offset}: expected 0x{expected:04X}, found 0x{found:04X}")
            }
            ParseError::Invalid { offset, what } => {
                write!(f, "invalid {what} at offset {offset}")
            }
            ParseError::BadString { offset } => {
                write!(f, "invalid UTF-8 string at offset {offset}")
            }
            ParseError::LimitExceeded { offset, what } => {
                write!(f, "safety limit exceeded at offset {offset}: {what}")
            }
        }
    }
}

impl std::error::Error for ParseError {}
