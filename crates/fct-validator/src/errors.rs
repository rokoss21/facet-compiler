//! # Validation Errors
//!
//! This module contains comprehensive error types for the FACET validator.

use thiserror::Error;

/// Result type for validation operations
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Comprehensive validation errors for the FACET compiler.
///
/// This enum represents all possible errors that can occur during validation,
/// including type errors, import issues, and semantic validation failures.
#[derive(Error, Debug)]
pub enum ValidationError {
    /// F401: Variable reference could not be resolved in the current scope.
    ///
    /// This error occurs when a variable is referenced but never declared,
    /// or is referenced outside its valid scope.
    #[error("F401: Variable not found: {var}")]
    #[allow(dead_code)] // TODO: Implement variable not found validation
    VariableNotFound {
        /// The name of the variable that could not be found
        var: String
    },

    /// F402: Type inference failed due to insufficient or conflicting information.
    ///
    /// This occurs when the validator cannot determine the type of a variable
    /// or expression, often due to circular dependencies or ambiguous usage.
    #[error("F402: Type inference failed: {message}")]
    TypeInferenceFailed {
        /// Detailed explanation of why type inference failed
        message: String
    },

    /// F404: Variable used before its declaration (forward reference).
    ///
    /// FACET requires variables to be declared before they are used.
    /// This error helps catch potential uninitialized variable usage.
    #[error("F404: Forward reference detected: variable {var} used before declaration")]
    ForwardReference {
        /// The name of the variable that was used forward-referenced
        var: String
    },

    /// F451: Type mismatch between expected and actual types.
    ///
    /// This is the most common type error, occurring when a value of one type
    /// is used where a different type is expected.
    #[error("F451: Type mismatch: expected {expected}, got {got} at {location}")]
    TypeMismatch {
        /// The type that was expected in this context
        expected: String,
        /// The actual type that was found
        got: String,
        /// Location information for debugging (file, line, context)
        location: String,
    },

    /// F452: Constraint violation for a type with additional restrictions.
    ///
    /// This occurs when a value violates type constraints like ranges,
    /// patterns, or enumerated values.
    #[error("F452: Constraint violation: {constraint} failed for value {value}")]
    ConstraintViolation {
        /// Description of the constraint that was violated
        constraint: String,
        /// The actual value that violated the constraint
        value: String
    },

    /// F453: Runtime input validation failed during dynamic checks.
    ///
    /// This error is used for validation failures that can only be detected
    /// at runtime, such as user input validation.
    #[error("F453: Runtime input validation failed: {message}")]
    InputValidationFailed {
        /// Details about why the input validation failed
        message: String
    },

    /// F601: Import path could not be resolved or file not found.
    ///
    /// This error occurs when an @import directive references a file
    /// that doesn't exist or cannot be accessed.
    #[error("F601: Import not found: {path}")]
    ImportNotFound {
        /// The import path that could not be resolved
        path: String
    },

    /// F602: Circular import dependency detected.
    ///
    /// This error occurs when files import each other in a cycle,
    /// creating a circular dependency that cannot be resolved.
    #[error("F602: Circular import detected: {path}")]
    CircularImport {
        /// The import path that completed the circular dependency
        path: String
    },

    /// F802: Lens function not found in the lens registry.
    ///
    /// This error occurs when a lens operation references a lens
    /// that is not registered or available.
    #[error("F802: Unknown lens: {lens_name}")]
    UnknownLens {
        /// The name of the lens that was not found
        lens_name: String
    },
}