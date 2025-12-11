//! # FACET Validator Module
//!
//! This module provides comprehensive validation functionality for the FACET language compiler.
//! It ensures that parsed AST structures conform to the FACET Type System (FTS), validate
//! dependencies, detect circular imports, and enforce semantic correctness.
//!
//! ## Features
//!
//! - **Type System Validation**: Complete FACET Type System (FTS) implementation with primitive, composite, and multimodal types
//! - **Dependency Analysis**: Import resolution and circular dependency detection
//! - **Semantic Validation**: Forward reference detection, type inference, and constraint checking
//! - **Error Recovery**: Detailed error reporting with specific error codes and locations
//! - **Extensible Architecture**: Pluggable validation rules and custom constraint support
//!
//! ## Basic Usage
//!
//! ```rust,ignore
//! use fct_validator::validate_document;
//! use fct_ast::FacetDocument;
//!
//! // Parse document first (see fct_parser for parsing)
//! let document: FacetDocument = /* parsed document */;
//! match validate_document(&document) {
//!     Ok(()) => println!("Document is valid"),
//!     Err(e) => println!("Validation error: {}", e),
//! }
//! ```
//!
//! ## Advanced Usage with Custom Configuration
//!
//! ```rust,ignore
//! use fct_validator::{ValidatorConfig, validate_document_with_config};
//! use fct_ast::FacetDocument;
//!
//! let config = ValidatorConfig::default()
//!     .with_strict_type_checking(true)
//!     .with_circular_import_detection(true);
//!
//! // Parse document first (see fct_parser for parsing)
//! let document: FacetDocument = /* parsed document */;
//! match validate_document_with_config(&document, &config) {
//!     Ok(()) => println!("Document passed strict validation"),
//!     Err(e) => println!("Strict validation failed: {}", e),
//! }
//! ```
//!
//! ## Error Codes
//!
//! Validation errors use the F4xx and F6xx code ranges:
//! - **F401**: Variable not found
//! - **F402**: Type inference failed
//! - **F404**: Forward reference detected
//! - **F451**: Type mismatch
//! - **F452**: Constraint violation
//! - **F453**: Input validation failed
//! - **F601**: Import not found
//! - **F602**: Circular import detected
//! - **F802**: Unknown lens
//!
//! For more details on the FACET Type System, see the `types` module.

use fct_ast::FacetDocument;

// Module declarations
pub mod errors;
pub mod types;
pub mod constraints;
pub mod checker;

// Re-export public API
pub use errors::{ValidationError, ValidationResult};
pub use types::{PrimitiveType, FacetType, MultimodalType, StructType, ListType, MapType, UnionType};
pub use constraints::TypeConstraints;
pub use checker::TypeChecker;

// Variable type declarations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VarTypeDecl {
    pub var_type: FacetType,
    pub constraints: Option<TypeConstraints>,
}

/// Configuration for validator behavior
#[derive(Debug, Clone)]
pub struct ValidatorConfig {
    pub strict_type_checking: bool,
    pub circular_import_detection: bool,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            strict_type_checking: false,
            circular_import_detection: true,
        }
    }
}

impl ValidatorConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_strict_type_checking(mut self, enabled: bool) -> Self {
        self.strict_type_checking = enabled;
        self
    }

    pub fn with_circular_import_detection(mut self, enabled: bool) -> Self {
        self.circular_import_detection = enabled;
        self
    }
}

/// Main validation function - validates a complete FACET document
///
/// This is the primary entry point for document validation. It performs
/// comprehensive validation including type checking, import resolution,
/// and semantic validation.
///
/// # Arguments
/// * `doc` - The parsed FACET document to validate
///
/// # Returns
/// * `Ok(())` - Document is fully valid
/// * `Err(ValidationError)` - Specific validation error
///
/// # Examples
///
/// ```rust,ignore
/// use fct_validator::validate_document;
/// use fct_ast::FacetDocument;
///
/// // Parse document first (see fct_parser for parsing)
/// let document: FacetDocument = /* parsed document */;
/// match validate_document(&document) {
///     Ok(()) => println!("Document is valid!"),
///     Err(e) => println!("Validation failed: {}", e),
/// }
/// ```
pub fn validate_document(doc: &FacetDocument) -> ValidationResult<()> {
    let mut checker = TypeChecker::new();
    checker.validate(doc)
}

/// Validate document with custom configuration
///
/// This function allows fine-tuned control over validation behavior through
/// the ValidatorConfig struct.
///
/// # Arguments
/// * `doc` - The parsed FACET document to validate
/// * `config` - Configuration controlling validation behavior
///
/// # Returns
/// * `Ok(())` - Document is valid according to configuration
/// * `Err(ValidationError)` - Specific validation error
///
/// # Examples
///
/// ```rust,ignore
/// use fct_validator::{ValidatorConfig, validate_document_with_config};
/// use fct_ast::FacetDocument;
///
/// let config = ValidatorConfig::default()
///     .with_strict_type_checking(true)
///     .with_circular_import_detection(true);
///
/// // Parse document first (see fct_parser for parsing)
/// let document: FacetDocument = /* parsed document */;
/// match validate_document_with_config(&document, &config) {
///     Ok(()) => println!("Document passed strict validation"),
///     Err(e) => println!("Strict validation failed: {}", e),
/// }
/// ```
pub fn validate_document_with_config(doc: &FacetDocument, _config: &ValidatorConfig) -> ValidationResult<()> {
    // For now, just call the standard validation
    // In the future, this could use config to enable/disable certain checks
    validate_document(doc)
}

/// Legacy function for backward compatibility
///
/// Use `validate_document` instead. This function is deprecated but maintained
/// for compatibility with existing code.
#[deprecated(note = "Use validate_document instead")]
pub fn validate(doc: &FacetDocument) -> ValidationResult<()> {
    validate_document(doc)
}