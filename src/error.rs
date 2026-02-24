//! Error types for document conversion.
//!
//! The primary error type is [`ConvertError`], which covers all failure modes
//! from unsupported formats to I/O and parsing errors.

/// Errors that can occur during document conversion.
///
/// Most conversion operations return `Result<ConversionResult, ConvertError>`.
/// For recoverable issues during best-effort conversion, see
/// [`ConversionWarning`](crate::ConversionWarning) instead.
#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    /// The file format is not recognized by any converter.
    #[error("unsupported format: {extension}")]
    UnsupportedFormat {
        /// The file extension or format identifier that was not recognized.
        extension: String,
    },

    /// The format is recognized but intentionally not supported, with a reason.
    #[error("{extension}: {reason}")]
    FormatNotSupported {
        /// The file extension or format identifier.
        extension: String,
        /// Explanation of why this format is not supported.
        reason: String,
    },

    /// The input data exceeds the configured size limit.
    #[error("input too large: {size} bytes exceeds limit of {limit} bytes")]
    InputTooLarge {
        /// Actual size of the input in bytes.
        size: usize,
        /// Maximum allowed size in bytes.
        limit: usize,
    },

    /// Failed to read or decompress a ZIP archive (DOCX, PPTX, XLSX).
    #[error("failed to read ZIP archive")]
    ZipError(#[from] zip::result::ZipError),

    /// Failed to parse XML content within a document.
    #[error("failed to parse XML")]
    XmlError(#[from] quick_xml::Error),

    /// Failed to read a spreadsheet file (XLSX or XLS).
    #[error("failed to read spreadsheet")]
    SpreadsheetError(#[from] calamine::Error),

    /// An I/O error occurred while reading the file.
    #[error("I/O error")]
    Io(#[from] std::io::Error),

    /// The input data is not valid UTF-8.
    #[error("invalid UTF-8 content")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    /// The document structure is malformed beyond recovery.
    #[error("malformed document: {reason}")]
    MalformedDocument {
        /// Description of the structural problem.
        reason: String,
    },

    /// An LLM image description call failed.
    #[error("image description failed: {reason}")]
    ImageDescriptionError {
        /// Description of the failure.
        reason: String,
    },
}
