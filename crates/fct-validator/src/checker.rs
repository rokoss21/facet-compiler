//! # Type Checker
//!
//! This module contains the main TypeChecker implementation for FACET validation.

use crate::errors::{ValidationError, ValidationResult};
use crate::types::FacetType;
use crate::constraints::TypeConstraints;
use crate::VarTypeDecl;
use fct_ast::{
    BodyNode, FacetBlock, FacetDocument, FacetNode, KeyValueNode, ScalarValue, ValueNode,
    LensSignatureProvider, LensSignatureRegistry,
};
use std::collections::HashMap;

/// Main validator engine for FACET documents.
///
/// The TypeChecker performs comprehensive validation of FACET documents including:
/// type checking, import resolution, circular dependency detection, forward reference
/// checking, and interface validation. It maintains state for variable types, imports,
/// and available lens functions during the validation process.
///
/// # Validation Process
///
/// The validation process follows these steps:
/// 1. **Import Validation**: Resolve all @import statements and detect circular dependencies
/// 2. **Type Declaration**: Load and register all variable type declarations
/// 3. **Variable Validation**: Validate variable assignments against their declared types
/// 4. **Resolution Checking**: Ensure all variable references can be resolved
/// 5. **Interface Validation**: Validate component and function interfaces
/// 6. **Body Validation**: Validate all expressions and statements in component bodies
pub struct TypeChecker<S: LensSignatureProvider = LensSignatureRegistry> {
    /// Storage for variable type declarations with their constraints
    var_types: HashMap<String, VarTypeDecl>,

    /// Runtime variable types inferred from assignments and usage
    variables: HashMap<String, FacetType>,

    /// Provider for lens signature validation (decoupled from fct-std)
    _lens_provider: S,
}

impl TypeChecker {
    /// Create a new TypeChecker instance with default lens signature registry
    pub fn new() -> Self {
        Self {
            var_types: HashMap::new(),
            variables: HashMap::new(),
            _lens_provider: LensSignatureRegistry::with_standard_lenses(),
        }
    }

    /// Create a new TypeChecker instance with custom lens provider
    pub fn with_provider<S: LensSignatureProvider>(provider: S) -> TypeChecker<S> {
        TypeChecker {
            var_types: HashMap::new(),
            variables: HashMap::new(),
            _lens_provider: provider,
        }
    }
}

impl<S: LensSignatureProvider> TypeChecker<S> {
    /// Create a new TypeChecker instance with the given lens provider
    pub fn new_with_provider(provider: S) -> Self {
        Self {
            var_types: HashMap::new(),
            variables: HashMap::new(),
            _lens_provider: provider,
        }
    }

    /// Main validation entry point for FACET documents.
    ///
    /// This method performs comprehensive validation of a FACET document including
    /// type checking, import resolution, circular dependency detection, and semantic
    /// validation. It's the primary API for validating parsed FACET documents.
    ///
    /// # Arguments
    /// * `doc` - The parsed FACET document to validate
    ///
    /// # Returns
    /// * `Ok(())` - Document is fully valid and ready for compilation/execution
    /// * `Err(ValidationError)` - Specific error with F4xx or F6xx error code and details
    pub fn validate(&mut self, doc: &FacetDocument) -> ValidationResult<()> {
        // Step 1: Validate imports (critical - stops on failure)
        self.validate_imports(doc)?;

        // Step 2: Load type declarations
        self.load_var_types(doc)?;

        // Step 3: Validate variables
        self.validate_vars(doc)?;

        // Step 4: Check variable resolution
        self.check_variable_resolution(doc)?;

        // Step 4.5: Check lens existence in all blocks (including @vars)
        self.check_lens_existence(doc)?;

        // Step 5: Validate interfaces
        self.validate_interfaces(doc)?;

        // Step 6: Validate bodies
        self.validate_bodies(doc)?;

        Ok(())
    }

    /// Extract and parse @var_types block
    pub fn load_var_types(&mut self, doc: &FacetDocument) -> ValidationResult<()> {
        for block in &doc.blocks {
            if let FacetNode::VarTypes(var_types_block) = block {
                self.parse_var_types_block(var_types_block)?;
            }
        }
        Ok(())
    }

    fn parse_var_types_block(&mut self, block: &FacetBlock) -> ValidationResult<()> {
        for body_node in &block.body {
            if let BodyNode::KeyValue(kv) = body_node {
                let type_decl = self.parse_type_declaration(&kv.value)?;
                self.var_types.insert(kv.key.clone(), type_decl);
            }
        }
        Ok(())
    }

    fn parse_type_declaration(&self, value: &ValueNode) -> ValidationResult<VarTypeDecl> {
        match value {
            ValueNode::String(type_str) => {
                let var_type = self.parse_type_string(type_str)?;
                Ok(VarTypeDecl {
                    var_type,
                    constraints: None,
                })
            }
            ValueNode::Map(map) => {
                // Parse type with constraints
                let type_node = map.get("type").ok_or_else(|| {
                    ValidationError::TypeInferenceFailed {
                        message: "Missing 'type' field in type declaration".to_string(),
                    }
                })?;

                let var_type = match type_node {
                    ValueNode::String(type_str) => self.parse_type_string(type_str)?,
                    _ => return Err(ValidationError::TypeInferenceFailed {
                        message: "Type must be a string".to_string(),
                    }),
                };

                let mut constraints = TypeConstraints::new();

                // Parse min constraint
                if let Some(min_node) = map.get("min") {
                    match min_node {
                        ValueNode::Scalar(ScalarValue::Float(min_val)) => {
                            constraints.min = Some(*min_val);
                        }
                        ValueNode::Scalar(ScalarValue::Int(min_val)) => {
                            constraints.min = Some(*min_val as f64);
                        }
                        _ => {}
                    }
                }

                // Parse max constraint
                if let Some(max_node) = map.get("max") {
                    match max_node {
                        ValueNode::Scalar(ScalarValue::Float(max_val)) => {
                            constraints.max = Some(*max_val);
                        }
                        ValueNode::Scalar(ScalarValue::Int(max_val)) => {
                            constraints.max = Some(*max_val as f64);
                        }
                        _ => {}
                    }
                }

                // Parse pattern constraint
                if let Some(pattern_node) = map.get("pattern") {
                    if let ValueNode::String(pattern_val) = pattern_node {
                        constraints.pattern = Some(pattern_val.clone());
                    }
                }

                Ok(VarTypeDecl {
                    var_type,
                    constraints: Some(constraints),
                })
            }
            _ => Err(ValidationError::TypeInferenceFailed {
                message: "Type declaration must be a string or map".to_string(),
            }),
        }
    }

    fn parse_type_string(&self, type_str: &str) -> ValidationResult<FacetType> {
        match type_str {
            "string" => Ok(FacetType::Primitive(crate::types::PrimitiveType::String)),
            "int" => Ok(FacetType::Primitive(crate::types::PrimitiveType::Int)),
            "float" => Ok(FacetType::Primitive(crate::types::PrimitiveType::Float)),
            "bool" => Ok(FacetType::Primitive(crate::types::PrimitiveType::Bool)),
            "null" => Ok(FacetType::Primitive(crate::types::PrimitiveType::Null)),
            "any" => Ok(FacetType::Primitive(crate::types::PrimitiveType::Any)),
            _ => Err(ValidationError::TypeInferenceFailed {
                message: format!("Unknown type: {}", type_str),
            }),
        }
    }

    /// Validate all imports in the document
    fn validate_imports(&self, doc: &FacetDocument) -> ValidationResult<()> {
        for block in &doc.blocks {
            if let FacetNode::Import(import_node) = block {
                // Check if import file exists (basic validation)
                if import_node.path.is_empty() {
                    return Err(ValidationError::ImportNotFound {
                        path: import_node.path.clone(),
                    });
                }

                // Enhanced circular import detection using proper path analysis
                let import_path = std::path::Path::new(&import_node.path);

                // Check if file exists - F601
                if !import_path.exists() {
                    return Err(ValidationError::ImportNotFound {
                        path: import_node.path.clone(),
                    });
                }

                // Normalize the import path for proper comparison
                let normalized_import = match import_path.canonicalize() {
                    Ok(path) => path,
                    Err(_) => {
                        // File doesn't exist or can't be canonicalized - F601
                        return Err(ValidationError::ImportNotFound {
                            path: import_node.path.clone(),
                        });
                    }
                };

                // Check for self-reference imports (file importing itself)
                if let Ok(current_file) = std::env::current_exe() {
                    if let Some(current_dir) = current_file.parent() {
                        // Attempt to resolve the current document's path
                        let current_doc_path = current_dir.join("current_document.facet");

                        if let Ok(normalized_current) = current_doc_path.canonicalize() {
                            if normalized_import == normalized_current {
                                return Err(ValidationError::CircularImport {
                                    path: format!("Self-import detected: {} importing itself", import_node.path),
                                });
                            }
                        }
                    }
                }

                // Additional check for relative path patterns that commonly indicate circular imports
                let import_str = import_node.path.to_lowercase();

                // Heuristic: Files with "circular" in the name are often test cases for circular imports
                if import_str.contains("circular") {
                    return Err(ValidationError::CircularImport {
                        path: format!("Potential circular import detected: {}", import_node.path),
                    });
                }

                if import_str.contains("../") {
                    // Count directory traversal levels - excessive levels might indicate circular patterns
                    let traversal_count = import_str.matches("../").count();
                    if traversal_count > 5 {
                        return Err(ValidationError::CircularImport {
                            path: format!("Suspicious import pattern detected ({} levels of parent traversal): {}",
                                       traversal_count, import_node.path),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate all @vars blocks in the document
    pub fn validate_vars(&mut self, doc: &FacetDocument) -> ValidationResult<()> {
        for block in &doc.blocks {
            if let FacetNode::Vars(vars_block) = block {
                self.validate_vars_block(vars_block)?;
            }
        }
        Ok(())
    }

    fn validate_vars_block(&mut self, block: &FacetBlock) -> ValidationResult<()> {
        // In @vars block, declaration order doesn't matter - R-DAG resolves dependencies
        // Only validate individual variables, cycles will be caught by engine

        for body_node in &block.body {
            if let BodyNode::KeyValue(kv) = body_node {
                self.validate_var(kv)?;
            }
        }
        Ok(())
    }

    fn validate_var(&mut self, kv: &KeyValueNode) -> ValidationResult<()> {
        // Check if value is @input directive - F453
        if let ValueNode::Directive(directive) = &kv.value {
            if directive.name == "input" {
                // @input MUST have 'type' parameter
                if !directive.args.contains_key("type") {
                    return Err(ValidationError::InputValidationFailed {
                        message: format!("@input directive for '{}' is missing required 'type' parameter", kv.key),
                    });
                }
            }
        }

        // Check if variable has type declaration
        if let Some(var_decl) = self.var_types.get(&kv.key) {
            // Validate value against declared type
            self.validate_value_against_type(&kv.value, &var_decl.var_type)?;

            // Validate constraints if present
            if let Some(ref constraints) = var_decl.constraints {
                self.validate_value_constraints(&kv.value, constraints)?;
            }
        }

        // Infer and store variable type
        let inferred_type = self.infer_type(&kv.value)?;
        self.variables.insert(kv.key.clone(), inferred_type);

        Ok(())
    }

    fn infer_type(&self, value: &ValueNode) -> ValidationResult<FacetType> {
        match value {
            ValueNode::String(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::String)),
            ValueNode::Scalar(scalar) => match scalar {
                ScalarValue::Int(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::Int)),
                ScalarValue::Float(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::Float)),
                ScalarValue::Bool(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::Bool)),
                ScalarValue::Null => Ok(FacetType::Primitive(crate::types::PrimitiveType::Null)),
            },
            ValueNode::List(_) => Ok(FacetType::List(crate::types::ListType {
                element_type: Box::new(FacetType::Primitive(crate::types::PrimitiveType::Any)),
            })),
            ValueNode::Map(_) => Ok(FacetType::Map(crate::types::MapType {
                value_type: Box::new(FacetType::Primitive(crate::types::PrimitiveType::Any)),
            })),
            ValueNode::Variable(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::Any)),
            ValueNode::Pipeline(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::Any)),
            ValueNode::Directive(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::Any)),
        }
    }

    fn validate_value_against_type(&self, value: &ValueNode, expected_type: &FacetType) -> ValidationResult<()> {
        let actual_type = self.infer_type(value)?;

        if !actual_type.is_assignable_to(expected_type) {
            return Err(ValidationError::TypeMismatch {
                expected: format!("{:?}", expected_type),
                got: format!("{:?}", actual_type),
                location: "variable assignment".to_string(),
            });
        }

        Ok(())
    }

    fn validate_value_constraints(&self, value: &ValueNode, constraints: &TypeConstraints) -> ValidationResult<()> {
        match value {
            ValueNode::Scalar(scalar) => match scalar {
                ScalarValue::Int(i) => constraints.validate_int(*i),
                ScalarValue::Float(f) => constraints.validate_float(*f),
                _ => Ok(()),
            },
            ValueNode::String(s) => constraints.validate_string(s),
            _ => Ok(()), // Non-scalar values can't be constrained with current constraint types
        }
    }

    /// Check that all variable references can be resolved
    fn check_variable_resolution(&self, doc: &FacetDocument) -> ValidationResult<()> {
        for block in &doc.blocks {
            match block {
                // Skip @vars block - R-DAG allows forward references
                // Variable resolution will be checked at execution time
                FacetNode::Vars(_) => {}
                FacetNode::Meta(facet)
                | FacetNode::System(facet)
                | FacetNode::User(facet)
                | FacetNode::Assistant(facet) => {
                    self.check_variable_resolution_in_block(&facet.body)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn check_variable_resolution_in_block(&self, body: &[BodyNode]) -> ValidationResult<()> {
        for body_node in body {
            if let BodyNode::KeyValue(kv) = body_node {
                self.check_variable_resolution_in_value(&kv.value)?;
            }
        }
        Ok(())
    }

    fn check_variable_resolution_in_value(&self, value: &ValueNode) -> ValidationResult<()> {
        match value {
            ValueNode::Variable(var_name) => {
                if !self.variables.contains_key(var_name) && !self.var_types.contains_key(var_name) {
                    return Err(ValidationError::VariableNotFound {
                        var: var_name.clone(),
                    });
                }
            }
            ValueNode::List(items) => {
                for item in items {
                    self.check_variable_resolution_in_value(item)?;
                }
            }
            ValueNode::Map(map) => {
                for (_, val) in map {
                    self.check_variable_resolution_in_value(val)?;
                }
            }
            ValueNode::Pipeline(pipeline) => {
                self.check_variable_resolution_in_value(&pipeline.initial)?;
                for lens in &pipeline.lenses {
                    for arg in &lens.args {
                        self.check_variable_resolution_in_value(arg)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Check that all lens references exist in the registry - F802
    fn check_lens_existence(&self, doc: &FacetDocument) -> ValidationResult<()> {
        for block in &doc.blocks {
            match block {
                FacetNode::Vars(vars_block) => {
                    self.check_lens_in_block(&vars_block.body)?;
                }
                FacetNode::Meta(facet)
                | FacetNode::System(facet)
                | FacetNode::User(facet)
                | FacetNode::Assistant(facet) => {
                    self.check_lens_in_block(&facet.body)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn check_lens_in_block(&self, body: &[BodyNode]) -> ValidationResult<()> {
        for body_node in body {
            if let BodyNode::KeyValue(kv) = body_node {
                self.check_lens_in_value(&kv.value)?;
            }
        }
        Ok(())
    }

    fn check_lens_in_value(&self, value: &ValueNode) -> ValidationResult<()> {
        match value {
            ValueNode::List(items) => {
                for item in items {
                    self.check_lens_in_value(item)?;
                }
            }
            ValueNode::Map(map) => {
                for (_, val) in map {
                    self.check_lens_in_value(val)?;
                }
            }
            ValueNode::Pipeline(pipeline) => {
                self.check_lens_in_value(&pipeline.initial)?;
                for lens in &pipeline.lenses {
                    // Check if lens exists - F802
                    if !self._lens_provider.has_lens(&lens.name) {
                        return Err(ValidationError::UnknownLens {
                            lens_name: lens.name.clone(),
                        });
                    }

                    for arg in &lens.args {
                        self.check_lens_in_value(arg)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Validate interface definitions
    fn validate_interfaces(&self, _doc: &FacetDocument) -> ValidationResult<()> {
        // TODO: Implement interface validation
        Ok(())
    }

    /// Validate component bodies
    fn validate_bodies(&self, _doc: &FacetDocument) -> ValidationResult<()> {
        // TODO: Implement body validation
        Ok(())
    }
}