use thiserror::Error;
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParserError {
    #[error("F001: Invalid indentation at line {0}")]
    InvalidIndentation(usize),
    #[error("F002: Tabs are not allowed (line {0})")]
    TabsNotAllowed(usize),
    #[error("F003: Unclosed delimiter {0} at line {1}")]
    UnclosedDelimiter(String, usize),
    #[error("F601: Import not found: {path}")]
    ImportNotFound { path: String },
    #[error("F602: Circular import detected: {chain}")]
    CircularImport { chain: String },
    #[error("Syntax error: {0}")]
    NomError(String),
}

pub type ParseResult<'a, T> =
    nom::IResult<SpanInput<'a>, T, nom::error::VerboseError<SpanInput<'a>>>;

pub type SpanInput<'a> = nom_locate::LocatedSpan<&'a str>;
