use miette::SourceSpan;
use std::ops::Deref;

/// A wrapper type that holds a value and its source span
/// This is used to provide rich error messages with source context
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spanned<T> {
    value: T,
    span: SourceSpan,
}

impl<T> Spanned<T> {
    /// Create a new spanned value
    pub fn new(value: T, span: SourceSpan) -> Self {
        Self { value, span }
    }

    /// Create a spanned value from byte offsets
    pub fn with_offsets(value: T, start: usize, end: usize) -> Self {
        Self {
            value,
            span: SourceSpan::from((start, end - start)),
        }
    }

    /// Get a reference to the value
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Get a reference to the span
    pub fn span(&self) -> SourceSpan {
        self.span
    }

    /// Consume self and return the value
    pub fn into_value(self) -> T {
        self.value
    }

    /// Map the value while preserving the span
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Spanned<U> {
        Spanned::new(f(self.value), self.span)
    }

    /// Try to map the value, propagating errors while preserving the span info
    pub fn try_map<U, E, F: FnOnce(T) -> Result<U, E>>(
        self,
        f: F,
    ) -> Result<Spanned<U>, E> {
        Ok(Spanned::new(f(self.value)?, self.span))
    }

    /// Combine two spanned values into a tuple, using the first span
    pub fn zip<U>(self, other: Spanned<U>) -> Spanned<(T, U)> {
        Spanned::new((self.value, other.value), self.span)
    }

    /// Create a spanned value covering the range from this span to another
    pub fn join<U>(&self, other: &Spanned<U>) -> SourceSpan {
        let start = self.span.offset();
        let end = other.span.offset() + other.span.len();
        SourceSpan::from((start, end - start))
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Spanned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}
