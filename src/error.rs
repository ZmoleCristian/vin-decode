use thiserror::Error;

/// Specialized [`Result`] alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced during VIN parsing and decoding.
#[derive(Debug, Error)]
pub enum Error {
    /// VIN length isn't 17.
    #[error("VIN must be 17 chars, got {0}")]
    InvalidLength(usize),

    /// VIN contains a forbidden character (`I`, `O`, or `Q`).
    #[error("VIN contains forbidden char `{0}` (I, O, Q not allowed)")]
    ForbiddenChar(char),

    /// VIN contains a non-ASCII-alphanumeric character.
    #[error("VIN contains non-ascii-alnum char `{0}`")]
    InvalidChar(char),

    /// Computed check digit doesn't match the one in the VIN.
    #[error("check digit mismatch: expected `{expected}`, got `{actual}`")]
    BadCheckDigit {
        /// Check digit computed from the VIN body.
        expected: char,
        /// Check digit found at position 9 of the VIN.
        actual: char,
    },

    /// WMI prefix has no entry in the lookup tables.
    #[error("unknown WMI: `{0}`")]
    UnknownWmi(String),

    /// Year code at position 10 isn't a valid model-year code.
    #[error("unreadable model year (code `{0}`)")]
    UnreadableYear(char),

    /// Required data file couldn't be opened.
    #[error("map data missing at `{0}`")]
    MissingData(String),

    /// Wraps underlying I/O errors.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
