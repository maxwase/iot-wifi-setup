use core::fmt;
use std::error::Error;

/// An [Error] formatter that concatenates all error's sources with `: `.
pub struct OneLineFormatter<E: Error>(E);

impl<E: Error> OneLineFormatter<E> {
    /// Wraps the error with formatter.
    pub fn new(e: E) -> Self {
        Self(e)
    }
}

impl<E: Error> fmt::Display for OneLineFormatter<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)?;

        let mut source = self.0.source();
        while let Some(cause) = source {
            write!(f, ": {cause}")?;
            source = cause.source();
        }
        Ok(())
    }
}
