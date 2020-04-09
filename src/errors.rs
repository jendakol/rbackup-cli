use err_derive::Error;

#[derive(Debug, Error)]
pub enum HttpError {
    #[error(
        display = "invalid HTTP status (expected: {:?}, got: {:?})",
        expected,
        found
    )]
    InvalidStatus { expected: u16, found: u16 },
    // #[error(display = "invalid response: {:?}", _0)]
    // InvalidResponse(String),
}
