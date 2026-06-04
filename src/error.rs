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

#[derive(Debug, Clone)]
pub enum Location {
    Span(Span),
    Bar { bar: usize, note: usize },
}

#[derive(Debug)]
pub struct JianPuError {
    pub location: Location,
    pub message: String,
}

impl JianPuError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self { location: Location::Span(span), message: message.into() }
    }

    pub fn at_bar(bar: usize, note: usize, message: impl Into<String>) -> Self {
        Self { location: Location::Bar { bar, note }, message: message.into() }
    }
}

impl std::fmt::Display for JianPuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.location {
            Location::Span(span) => write!(f, "{} (at byte {}-{})", self.message, span.start, span.end),
            Location::Bar { bar, note: 0 } => write!(f, "{} (bar {})", self.message, bar),
            Location::Bar { bar, note } => write!(f, "{} (bar {}, note {})", self.message, bar, note),
        }
    }
}

impl std::error::Error for JianPuError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bar_error_display_with_note() {
        let e = JianPuError::at_bar(3, 2, "too many beats");
        assert_eq!(format!("{}", e), "too many beats (bar 3, note 2)");
    }

    #[test]
    fn bar_error_display_whole_bar() {
        let e = JianPuError::at_bar(5, 0, "incomplete measure");
        assert_eq!(format!("{}", e), "incomplete measure (bar 5)");
    }

    #[test]
    fn span_error_display() {
        let e = JianPuError::new(Span::new(10, 20), "bad token");
        assert_eq!(format!("{}", e), "bad token (at byte 10-20)");
    }
}
