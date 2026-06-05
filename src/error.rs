use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Span {
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

#[derive(Debug)]
pub struct JianPuError {
    pub span: Span,
    pub message: String,
    pub path: Option<PathBuf>,
}

impl JianPuError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
            path: None,
        }
    }

    pub fn with_path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }
}

impl std::fmt::Display for JianPuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error: {}", self.message)
    }
}

impl std::error::Error for JianPuError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_shows_message() {
        let e = JianPuError::new(Span::new(10, 20), "bad token");
        assert_eq!(format!("{}", e), "error: bad token");
    }

    #[test]
    fn with_path_attaches_path() {
        let e = JianPuError::new(Span::new(0, 1), "oops").with_path("/tmp/test.jianpu");
        assert_eq!(e.path.unwrap().to_str().unwrap(), "/tmp/test.jianpu");
    }

    #[test]
    fn without_path_is_none() {
        let e = JianPuError::new(Span::new(0, 1), "oops");
        assert!(e.path.is_none());
    }
}
