use thiserror::Error;

/// Errors produced while parsing or routing an HTTP request.
#[derive(Debug, Error)]
pub enum Error {
    /// Input did not have the shape the parser required — a request line with no
    /// method, an unrecognised HTTP verb, or a path whose segment count does not
    /// match the route pattern it was tested against.
    ///
    /// The payload is a static description of what was expected, and is what the
    /// [`std::fmt::Display`] implementation renders.
    ///
    /// # Examples
    ///
    /// ```
    /// use error::Error;
    ///
    /// let err = Error::InvalidData("Invalid request lines");
    /// assert_eq!(err.to_string(), "Invalid request lines");
    /// ```
    #[error("{0}")]
    InvalidData(&'static str),
}

#[cfg(test)]
mod tests;
