use std::fmt;

use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum CompileErrorKind {
    #[error("Unexpected character")]
    UnclosedCharacter,
    #[error("Unclosed left bracket")]
    UnclosedLeftBracket,
    #[error("Unexpected right bracket")]
    UnexpectedRightBracket,
}

#[derive(Debug, Error)]
pub struct CompileError {
    pub(crate) line: usize,
    pub(crate) col: usize,
    pub(crate) kind: CompileErrorKind,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at line {}:{}", self.kind, self.line, self.col)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("IO: {0}")]
    IO(#[from] std::io::Error),

    #[error("overflow")]
    Overflow,
}
