#[derive(Debug, Clone)]
pub struct Span {
    /// Byte offset in the original source string.
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{message} (at byte {}-{})", span.start, span.end)]
pub struct JianPuError {
    pub span: Span,
    pub message: String,
}

impl JianPuError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self { span, message: message.into() }
    }
}
