//! # Error Types
//!
//! Structured error types for calc_core. These errors are designed to be
//! informative for both humans and LLMs, providing enough context to
//! understand and fix issues programmatically.
//!
//! ## Example
//!
//! ```rust
//! use calc_core::errors::{CalcError, CalcResult};
//!
//! fn validate_span(span_ft: f64) -> CalcResult<()> {
//!     if span_ft <= 0.0 {
//!         return Err(CalcError::InvalidInput {
//!             field: "span_ft".to_string(),
//!             value: span_ft.to_string(),
//!             reason: "Span must be positive".to_string(),
//!         });
//!     }
//!     Ok(())
//! }
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Result type alias for calc_core operations
pub type CalcResult<T> = Result<T, CalcError>;

/// Structured error type for calculation operations.
///
/// Each variant provides specific context about what went wrong,
/// enabling programmatic error handling by LLMs and other consumers.
#[derive(Error, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "details")]
pub enum CalcError {
    /// An input value is invalid (out of range, wrong type, etc.)
    #[error("Invalid input for '{field}': {value} - {reason}")]
    InvalidInput {
        field: String,
        value: String,
        reason: String,
    },

    /// A required field is missing
    #[error("Missing required field: {field}")]
    MissingField { field: String },

    /// Material not found in database
    #[error("Material not found: {material_name}")]
    MaterialNotFound { material_name: String },

    /// Calculation failed (member overstressed, unstable, etc.)
    #[error("Calculation failed: {calculation_type} - {reason}")]
    CalculationFailed {
        calculation_type: String,
        reason: String,
    },

    /// File I/O error
    #[error("File error: {operation} on '{path}' - {reason}")]
    FileError {
        operation: String,
        path: String,
        reason: String,
    },

    /// File is locked by another user/process
    #[error("File locked: '{path}' is locked by {locked_by} since {locked_at}")]
    FileLocked {
        path: String,
        locked_by: String,
        locked_at: String,
    },

    /// JSON serialization/deserialization error
    #[error("Serialization error: {reason}")]
    SerializationError { reason: String },

    /// Schema version mismatch
    #[error("Version mismatch: file version {file_version}, expected {expected_version}")]
    VersionMismatch {
        file_version: String,
        expected_version: String,
    },

    /// Generic internal error (should be rare)
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl CalcError {
    /// Create an InvalidInput error
    pub fn invalid_input(
        field: impl Into<String>,
        value: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        CalcError::InvalidInput {
            field: field.into(),
            value: value.into(),
            reason: reason.into(),
        }
    }

    /// Create a MissingField error
    pub fn missing_field(field: impl Into<String>) -> Self {
        CalcError::MissingField {
            field: field.into(),
        }
    }

    /// Create a MaterialNotFound error
    pub fn material_not_found(material_name: impl Into<String>) -> Self {
        CalcError::MaterialNotFound {
            material_name: material_name.into(),
        }
    }

    /// Create a CalculationFailed error
    pub fn calculation_failed(
        calculation_type: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        CalcError::CalculationFailed {
            calculation_type: calculation_type.into(),
            reason: reason.into(),
        }
    }

    /// Create a FileError
    pub fn file_error(
        operation: impl Into<String>,
        path: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        CalcError::FileError {
            operation: operation.into(),
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a FileLocked error
    pub fn file_locked(
        path: impl Into<String>,
        locked_by: impl Into<String>,
        locked_at: impl Into<String>,
    ) -> Self {
        CalcError::FileLocked {
            path: path.into(),
            locked_by: locked_by.into(),
            locked_at: locked_at.into(),
        }
    }

    /// Check if this is a recoverable error (e.g., can retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(self, CalcError::FileLocked { .. })
    }

    /// Get a short error code for programmatic handling
    pub fn error_code(&self) -> &'static str {
        match self {
            CalcError::InvalidInput { .. } => "INVALID_INPUT",
            CalcError::MissingField { .. } => "MISSING_FIELD",
            CalcError::MaterialNotFound { .. } => "MATERIAL_NOT_FOUND",
            CalcError::CalculationFailed { .. } => "CALCULATION_FAILED",
            CalcError::FileError { .. } => "FILE_ERROR",
            CalcError::FileLocked { .. } => "FILE_LOCKED",
            CalcError::SerializationError { .. } => "SERIALIZATION_ERROR",
            CalcError::VersionMismatch { .. } => "VERSION_MISMATCH",
            CalcError::Internal { .. } => "INTERNAL_ERROR",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_serialization() {
        let error = CalcError::invalid_input("span_ft", "−5.0", "Span must be positive");
        let json = serde_json::to_string(&error).unwrap();
        let roundtrip: CalcError = serde_json::from_str(&json).unwrap();
        assert_eq!(error, roundtrip);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(
            CalcError::missing_field("test").error_code(),
            "MISSING_FIELD"
        );
        assert_eq!(
            CalcError::material_not_found("steel").error_code(),
            "MATERIAL_NOT_FOUND"
        );
    }
}
