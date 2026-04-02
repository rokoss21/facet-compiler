//! # Type Checker
//!
//! This module contains the main TypeChecker implementation for FACET validation.

use crate::constraints::TypeConstraints;
use crate::errors::{ValidationError, ValidationResult};
use crate::types::FacetType;
use crate::VarTypeDecl;
use fct_ast::types::FacetType as AstFacetType;
use fct_ast::{
    BodyNode, FacetBlock, FacetDocument, FacetNode, KeyValueNode, LensSignatureProvider,
    LensSignatureRegistry, MapKeyKind, OrderedMap, ScalarValue, TypeNode, ValueNode,
};
use fct_resolver::{Resolver, ResolverConfig, ResolverError};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationProfile {
    Core,
    Hypervisor,
}

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

    /// Active validation profile
    profile: ValidationProfile,
}

impl TypeChecker {
    /// Create a new TypeChecker instance with default lens signature registry
    pub fn new() -> Self {
        Self {
            var_types: HashMap::new(),
            variables: HashMap::new(),
            _lens_provider: LensSignatureRegistry::with_standard_lenses(),
            profile: ValidationProfile::Hypervisor,
        }
    }

    /// Create a new TypeChecker instance with custom lens provider
    pub fn with_provider<S: LensSignatureProvider>(provider: S) -> TypeChecker<S> {
        TypeChecker {
            var_types: HashMap::new(),
            variables: HashMap::new(),
            _lens_provider: provider,
            profile: ValidationProfile::Hypervisor,
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: LensSignatureProvider> TypeChecker<S> {
    /// Create a new TypeChecker instance with the given lens provider
    pub fn new_with_provider(provider: S) -> Self {
        Self {
            var_types: HashMap::new(),
            variables: HashMap::new(),
            _lens_provider: provider,
            profile: ValidationProfile::Hypervisor,
        }
    }

    pub fn with_profile(mut self, profile: ValidationProfile) -> Self {
        self.profile = profile;
        self
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
        self.enforce_profile(doc)?;

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

        // Step 4.6: Validate lens pipeline step type assignability (F451)
        self.check_lens_pipeline_types(doc)?;

        // Step 4.75: Validate @policy schema and condition typing constraints
        self.validate_policy(doc)?;

        // Step 5: Validate interfaces
        self.validate_interfaces(doc)?;

        // Step 6: Validate bodies
        self.validate_bodies(doc)?;

        Ok(())
    }

    fn enforce_profile(&self, doc: &FacetDocument) -> ValidationResult<()> {
        if self.profile == ValidationProfile::Hypervisor {
            return Ok(());
        }

        for block in &doc.blocks {
            match block {
                FacetNode::Interface(_) => {
                    return Err(ValidationError::ProfileViolation {
                        construct: "@interface".to_string(),
                    });
                }
                FacetNode::Test(_) => {
                    return Err(ValidationError::ProfileViolation {
                        construct: "@test".to_string(),
                    });
                }
                FacetNode::Vars(vars_block) => {
                    for item in &vars_block.body {
                        if let BodyNode::KeyValue(kv) = item {
                            if !Self::is_core_literal_value(&kv.value) {
                                return Err(ValidationError::ProfileViolation {
                                    construct: format!("@vars.{}", kv.key),
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn is_core_literal_value(value: &ValueNode) -> bool {
        match value {
            ValueNode::Scalar(_) | ValueNode::String(_) => true,
            ValueNode::List(items) => items.iter().all(Self::is_core_literal_value),
            ValueNode::Map(map) => map.values().all(Self::is_core_literal_value),
            ValueNode::Variable(_) | ValueNode::Pipeline(_) | ValueNode::Directive(_) => false,
        }
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
                let type_node =
                    map.get("type")
                        .ok_or_else(|| ValidationError::TypeInferenceFailed {
                            message: "Missing 'type' field in type declaration".to_string(),
                        })?;

                let var_type = match type_node {
                    ValueNode::String(type_str) => self.parse_type_string(type_str)?,
                    _ => {
                        return Err(ValidationError::TypeInferenceFailed {
                            message: "Type must be a string".to_string(),
                        })
                    }
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
                if let Some(ValueNode::String(pattern_val)) = map.get("pattern") {
                    constraints.pattern = Some(pattern_val.clone());
                }

                // Parse enum constraint
                if let Some(enum_node) = map.get("enum") {
                    let enum_values = match enum_node {
                        ValueNode::List(items) if !items.is_empty() => items,
                        ValueNode::List(_) => {
                            return Err(ValidationError::ConstraintViolation {
                                constraint: "enum must be a non-empty list".to_string(),
                                value: "[]".to_string(),
                            });
                        }
                        other => {
                            return Err(ValidationError::ConstraintViolation {
                                constraint: "enum must be a list of atom literals".to_string(),
                                value: format!("{:?}", other),
                            });
                        }
                    };

                    if !enum_values.iter().all(Self::is_atom_value) {
                        return Err(ValidationError::ConstraintViolation {
                            constraint: "enum items must be atom literals".to_string(),
                            value: format!("{:?}", enum_values),
                        });
                    }

                    constraints.enum_values = Some(enum_values.clone());
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
        parse_type_expr(type_str)
    }

    /// Validate all imports in the document
    fn validate_imports(&self, doc: &FacetDocument) -> ValidationResult<()> {
        if !doc.blocks.iter().any(|b| matches!(b, FacetNode::Import(_))) {
            return Ok(());
        }

        let base_dir = std::env::current_dir().map_err(|e| ValidationError::ImportNotFound {
            path: e.to_string(),
        })?;

        let config = ResolverConfig {
            allowed_roots: vec![base_dir.clone()],
            base_dir,
        };
        let mut resolver = Resolver::new(config);

        resolver
            .resolve(doc.clone())
            .map(|_| ())
            .map_err(Self::map_resolver_error)
    }

    fn map_resolver_error(err: ResolverError) -> ValidationError {
        match err {
            ResolverError::ImportCycle { cycle } => ValidationError::CircularImport { path: cycle },
            ResolverError::ImportNotFound { path } => ValidationError::ImportNotFound { path },
            ResolverError::AbsolutePathNotAllowed { path } => {
                ValidationError::ImportNotFound { path }
            }
            ResolverError::ParentTraversalNotAllowed { path } => {
                ValidationError::ImportNotFound { path }
            }
            ResolverError::SymlinkEscape {
                link_path,
                target_path,
            } => ValidationError::ImportNotFound {
                path: format!("{} -> {}", link_path, target_path),
            },
            ResolverError::SensitiveLocationAccess { path } => {
                ValidationError::ImportNotFound { path }
            }
            ResolverError::SuspiciousEncoding { path } => ValidationError::ImportNotFound { path },
            ResolverError::FileReadTimeout { path, .. } => ValidationError::ImportNotFound { path },
            ResolverError::Io(e) => ValidationError::ImportNotFound {
                path: e.to_string(),
            },
            ResolverError::ParseError(message) => ValidationError::ImportNotFound { path: message },
        }
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
        self.validate_input_usage_for_var(&kv.value, &kv.key)?;

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
                ScalarValue::Float(_) => {
                    Ok(FacetType::Primitive(crate::types::PrimitiveType::Float))
                }
                ScalarValue::Bool(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::Bool)),
                ScalarValue::Null => Ok(FacetType::Primitive(crate::types::PrimitiveType::Null)),
            },
            ValueNode::List(_) => Ok(FacetType::List(Box::new(FacetType::Primitive(
                crate::types::PrimitiveType::Any,
            )))),
            ValueNode::Map(_) => Ok(FacetType::Map(Box::new(FacetType::Primitive(
                crate::types::PrimitiveType::Any,
            )))),
            ValueNode::Variable(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::Any)),
            ValueNode::Pipeline(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::Any)),
            ValueNode::Directive(directive) => {
                if directive.name == "input" {
                    if let Some(ValueNode::String(type_str)) = directive.args.get("type") {
                        return self.parse_type_string(type_str);
                    }
                }
                Ok(FacetType::Primitive(crate::types::PrimitiveType::Any))
            }
        }
    }

    fn validate_value_against_type(
        &self,
        value: &ValueNode,
        expected_type: &FacetType,
    ) -> ValidationResult<()> {
        if value_matches_expected_type(value, expected_type, self)? {
            return Ok(());
        }

        let actual_type = self.infer_type(value)?;
        Err(ValidationError::TypeMismatch {
            expected: format!("{:?}", expected_type),
            got: format!("{:?}", actual_type),
            location: "variable assignment".to_string(),
        })
    }

    fn validate_value_constraints(
        &self,
        value: &ValueNode,
        constraints: &TypeConstraints,
    ) -> ValidationResult<()> {
        match value {
            ValueNode::Scalar(scalar) => match scalar {
                ScalarValue::Int(i) => constraints.validate_int(*i),
                ScalarValue::Float(f) => constraints.validate_float(*f),
                ScalarValue::Bool(b) => constraints.validate_bool(*b),
                ScalarValue::Null => constraints.validate_null(),
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
                FacetNode::Meta(facet) | FacetNode::User(facet) | FacetNode::Assistant(facet) => {
                    self.check_variable_resolution_in_block(&facet.body, false)?;
                }
                FacetNode::System(facet) => {
                    self.check_variable_resolution_in_block(&facet.body, true)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn check_variable_resolution_in_block(
        &self,
        body: &[BodyNode],
        allow_interface_tool_refs: bool,
    ) -> ValidationResult<()> {
        for body_node in body {
            if let BodyNode::KeyValue(kv) = body_node {
                if allow_interface_tool_refs && kv.key == "tools" {
                    continue;
                }
                self.check_variable_resolution_in_value(&kv.value)?;
            }
        }
        Ok(())
    }

    fn check_variable_resolution_in_value(&self, value: &ValueNode) -> ValidationResult<()> {
        match value {
            ValueNode::Variable(var_name) => {
                if !self.variables.contains_key(var_name) && !self.var_types.contains_key(var_name)
                {
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

    fn check_lens_pipeline_types(&self, doc: &FacetDocument) -> ValidationResult<()> {
        for block in &doc.blocks {
            match block {
                FacetNode::Vars(vars_block) => {
                    self.check_lens_pipeline_types_in_block(&vars_block.body, "@vars")?;
                }
                FacetNode::Meta(facet) => {
                    self.check_lens_pipeline_types_in_block(&facet.body, "@meta")?;
                }
                FacetNode::System(facet) => {
                    self.check_lens_pipeline_types_in_block(&facet.body, "@system")?;
                }
                FacetNode::User(facet) => {
                    self.check_lens_pipeline_types_in_block(&facet.body, "@user")?;
                }
                FacetNode::Assistant(facet) => {
                    self.check_lens_pipeline_types_in_block(&facet.body, "@assistant")?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn check_lens_pipeline_types_in_block(
        &self,
        body: &[BodyNode],
        block_name: &str,
    ) -> ValidationResult<()> {
        for body_node in body {
            if let BodyNode::KeyValue(kv) = body_node {
                let location = format!("{}.{}", block_name, kv.key);
                self.infer_pipeline_checked_type(&kv.value, &location)?;
            }
        }
        Ok(())
    }

    fn infer_pipeline_checked_type(
        &self,
        value: &ValueNode,
        location: &str,
    ) -> ValidationResult<FacetType> {
        match value {
            ValueNode::String(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::String)),
            ValueNode::Scalar(ScalarValue::Int(_)) => {
                Ok(FacetType::Primitive(crate::types::PrimitiveType::Int))
            }
            ValueNode::Scalar(ScalarValue::Float(_)) => {
                Ok(FacetType::Primitive(crate::types::PrimitiveType::Float))
            }
            ValueNode::Scalar(ScalarValue::Bool(_)) => {
                Ok(FacetType::Primitive(crate::types::PrimitiveType::Bool))
            }
            ValueNode::Scalar(ScalarValue::Null) => {
                Ok(FacetType::Primitive(crate::types::PrimitiveType::Null))
            }
            ValueNode::Variable(var_ref) => {
                let base = var_ref.split('.').next().unwrap_or(var_ref);
                Ok(self
                    .variables
                    .get(base)
                    .cloned()
                    .or_else(|| self.var_types.get(base).map(|decl| decl.var_type.clone()))
                    .unwrap_or(FacetType::Primitive(crate::types::PrimitiveType::Any)))
            }
            ValueNode::Directive(_) => Ok(FacetType::Primitive(crate::types::PrimitiveType::Any)),
            ValueNode::List(items) => {
                for item in items {
                    self.infer_pipeline_checked_type(item, location)?;
                }
                Ok(FacetType::List(Box::new(FacetType::Primitive(
                    crate::types::PrimitiveType::Any,
                ))))
            }
            ValueNode::Map(map) => {
                for nested in map.values() {
                    self.infer_pipeline_checked_type(nested, location)?;
                }
                Ok(FacetType::Map(Box::new(FacetType::Primitive(
                    crate::types::PrimitiveType::Any,
                ))))
            }
            ValueNode::Pipeline(pipeline) => {
                let mut current = self.infer_pipeline_checked_type(&pipeline.initial, location)?;

                for lens in &pipeline.lenses {
                    let signature =
                        self._lens_provider
                            .get_signature(&lens.name)
                            .ok_or_else(|| ValidationError::UnknownLens {
                                lens_name: lens.name.clone(),
                            })?;

                    let expected_input = Self::ast_type_to_validator_type(&signature.input_type);
                    if !Self::is_pipeline_assignable(&current, &expected_input) {
                        return Err(ValidationError::TypeMismatch {
                            expected: format!("{:?}", expected_input),
                            got: format!("{:?}", current),
                            location: format!("lens step '{}' in {}", lens.name, location),
                        });
                    }

                    for arg in &lens.args {
                        self.infer_pipeline_checked_type(arg, location)?;
                    }
                    for arg in lens.kwargs.values() {
                        self.infer_pipeline_checked_type(arg, location)?;
                    }

                    current = Self::ast_type_to_validator_type(&signature.output_type);
                }

                Ok(current)
            }
        }
    }

    fn is_pipeline_assignable(actual: &FacetType, expected: &FacetType) -> bool {
        if actual.is_assignable_to(expected) {
            return true;
        }

        matches!(
            actual,
            FacetType::Primitive(crate::types::PrimitiveType::Any)
        )
    }

    fn ast_type_to_validator_type(ty: &AstFacetType) -> FacetType {
        ty.clone()
    }

    /// Validate interface definitions
    fn validate_interfaces(&self, doc: &FacetDocument) -> ValidationResult<()> {
        let mut interface_names = HashSet::new();

        for block in &doc.blocks {
            if let FacetNode::Interface(interface) = block {
                if !interface_names.insert(interface.name.clone()) {
                    return Err(Self::policy_err(
                        "Duplicate interface name in resolved document",
                        &interface.name,
                    ));
                }

                let mut fn_names = HashSet::new();
                for func in &interface.functions {
                    if !fn_names.insert(func.name.clone()) {
                        return Err(Self::policy_err(
                            "Duplicate function name in interface",
                            &format!("{}.{}", interface.name, func.name),
                        ));
                    }

                    let effect = func.effect.as_ref().ok_or_else(|| {
                        ValidationError::InvalidEffectDeclaration {
                            message: format!(
                                "Missing required effect for {}.{}",
                                interface.name, func.name
                            ),
                        }
                    })?;
                    if !Self::is_valid_effect_class(effect) {
                        return Err(ValidationError::InvalidEffectDeclaration {
                            message: format!(
                                "Invalid effect '{}' for {}.{}",
                                effect, interface.name, func.name
                            ),
                        });
                    }

                    Self::validate_interface_type_mappable(
                        &func.return_type,
                        &format!("{}.{} return type", interface.name, func.name),
                    )?;

                    let mut param_names = HashSet::new();
                    for param in &func.params {
                        if !param_names.insert(param.name.clone()) {
                            return Err(Self::policy_err(
                                "Duplicate parameter name in function",
                                &format!("{}.{}.{}", interface.name, func.name, param.name),
                            ));
                        }
                        Self::validate_interface_type_mappable(
                            &param.type_node,
                            &format!("{}.{}.{} parameter", interface.name, func.name, param.name),
                        )?;
                    }
                }
            }
        }

        for block in &doc.blocks {
            if let FacetNode::System(system) = block {
                for body in &system.body {
                    let BodyNode::KeyValue(kv) = body else {
                        continue;
                    };
                    if kv.key != "tools" {
                        continue;
                    }

                    let tool_items = match &kv.value {
                        ValueNode::List(items) => items,
                        _ => return Err(Self::policy_err("@system.tools must be a list", "tools")),
                    };

                    for item in tool_items {
                        let iface_name = match item {
                            ValueNode::Variable(name) => name.as_str(),
                            _ => {
                                return Err(Self::policy_err(
                                    "@system.tools entries must be interface refs like $Name",
                                    "tools",
                                ))
                            }
                        };

                        if !interface_names.contains(iface_name) {
                            return Err(Self::policy_err(
                                "Unknown interface reference in @system.tools",
                                iface_name,
                            ));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate component bodies
    fn validate_bodies(&self, doc: &FacetDocument) -> ValidationResult<()> {
        for block in &doc.blocks {
            match block {
                FacetNode::Meta(meta) => self.validate_meta_block(meta)?,
                FacetNode::Context(context) => self.validate_context_block(context)?,
                FacetNode::System(system) => self.validate_message_block(system, true)?,
                FacetNode::User(user) | FacetNode::Assistant(user) => {
                    self.validate_message_block(user, false)?
                }
                FacetNode::Vars(vars_block) | FacetNode::VarTypes(vars_block) => {
                    for entry in &vars_block.body {
                        if let BodyNode::KeyValue(kv) = entry {
                            self.ensure_identifier_block_key(kv)?;
                            Self::validate_value_map_keys(&kv.value, false)?;
                        }
                    }
                }
                FacetNode::Policy(policy) => {
                    for entry in &policy.body {
                        if let BodyNode::KeyValue(kv) = entry {
                            self.ensure_identifier_block_key(kv)?;
                            Self::validate_value_map_keys(&kv.value, false)?;
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn validate_meta_block(&self, block: &FacetBlock) -> ValidationResult<()> {
        for body in &block.body {
            let kv = match body {
                BodyNode::KeyValue(kv) => kv,
                _ => {
                    return Err(Self::policy_err(
                        "@meta body must contain key/value entries only",
                        "@meta",
                    ))
                }
            };

            if Self::contains_control_chars(&kv.key) {
                return Err(Self::policy_err(
                    "@meta key contains control characters",
                    &kv.key,
                ));
            }

            if !Self::is_atom_value(&kv.value) {
                return Err(Self::policy_err(
                    "@meta values must be atoms (string|number|bool|null)",
                    &kv.key,
                ));
            }
        }

        Ok(())
    }

    fn validate_context_block(&self, block: &FacetBlock) -> ValidationResult<()> {
        let mut budget_seen = false;

        for body in &block.body {
            let kv = match body {
                BodyNode::KeyValue(kv) => kv,
                _ => {
                    return Err(Self::policy_err(
                        "@context body must contain key/value entries only",
                        "@context",
                    ))
                }
            };
            self.ensure_identifier_block_key(kv)?;

            match kv.key.as_str() {
                "budget" => {
                    budget_seen = true;
                    match &kv.value {
                        ValueNode::Scalar(ScalarValue::Int(v)) if *v >= 0 => {}
                        _ => {
                            return Err(Self::policy_err(
                                "@context.budget must be integer >= 0",
                                "budget",
                            ))
                        }
                    }
                }
                "defaults" => self.validate_context_defaults(&kv.value)?,
                _ => return Err(Self::policy_err("Unknown @context key", &kv.key)),
            }
        }

        if !budget_seen {
            return Err(Self::policy_err(
                "Missing required @context.budget",
                "budget",
            ));
        }

        Ok(())
    }

    fn validate_context_defaults(&self, defaults: &ValueNode) -> ValidationResult<()> {
        let map = match defaults {
            ValueNode::Map(map) => map,
            _ => {
                return Err(Self::policy_err(
                    "@context.defaults must be a map",
                    "defaults",
                ))
            }
        };

        for (key, value) in map {
            match key.as_str() {
                "priority" | "min" => match value {
                    ValueNode::Scalar(ScalarValue::Int(v)) if *v >= 0 => {}
                    _ => {
                        return Err(Self::policy_err(
                            "@context.defaults priority/min must be integer >= 0",
                            key,
                        ))
                    }
                },
                "grow" | "shrink" => match value {
                    ValueNode::Scalar(ScalarValue::Int(v)) if *v >= 0 => {}
                    ValueNode::Scalar(ScalarValue::Float(v)) if *v >= 0.0 => {}
                    _ => {
                        return Err(Self::policy_err(
                            "@context.defaults grow/shrink must be number >= 0",
                            key,
                        ))
                    }
                },
                _ => return Err(Self::policy_err("Unknown @context.defaults key", key)),
            }
        }

        Ok(())
    }

    fn validate_message_block(
        &self,
        block: &FacetBlock,
        allow_tools: bool,
    ) -> ValidationResult<()> {
        if let Some(when_attr) = block.attributes.get("when") {
            self.validate_when_atom(when_attr)?;
        }

        let mut content_seen = false;
        let allowed = if allow_tools {
            [
                "content", "id", "priority", "min", "grow", "shrink", "strategy", "when", "tools",
            ]
            .into_iter()
            .collect::<HashSet<_>>()
        } else {
            [
                "content", "id", "priority", "min", "grow", "shrink", "strategy", "when",
            ]
            .into_iter()
            .collect::<HashSet<_>>()
        };

        for body in &block.body {
            let kv = match body {
                BodyNode::KeyValue(kv) => kv,
                _ => {
                    return Err(Self::policy_err(
                        "Message block body must contain key/value entries only",
                        &block.name,
                    ))
                }
            };
            self.ensure_identifier_block_key(kv)?;

            if !allowed.contains(kv.key.as_str()) {
                return Err(Self::policy_err("Unknown message field", &kv.key));
            }

            match kv.key.as_str() {
                "content" => {
                    content_seen = true;
                    self.validate_message_content(&kv.value)?;
                }
                "id" => {
                    if !matches!(kv.value, ValueNode::String(_)) {
                        return Err(Self::policy_err("Message id must be string", "id"));
                    }
                }
                "priority" | "min" => {
                    if !matches!(kv.value, ValueNode::Scalar(ScalarValue::Int(_))) {
                        return Err(Self::policy_err(
                            "Message priority/min must be integer",
                            &kv.key,
                        ));
                    }
                }
                "grow" | "shrink" => {
                    if !matches!(
                        kv.value,
                        ValueNode::Scalar(ScalarValue::Int(_))
                            | ValueNode::Scalar(ScalarValue::Float(_))
                    ) {
                        return Err(Self::policy_err(
                            "Message grow/shrink must be number",
                            &kv.key,
                        ));
                    }
                }
                "strategy" => {
                    if !matches!(kv.value, ValueNode::Pipeline(_)) {
                        return Err(Self::policy_err(
                            "Message strategy must be a lens pipeline",
                            "strategy",
                        ));
                    }
                }
                "when" => self.validate_when_atom(&kv.value)?,
                "tools" => {
                    if !allow_tools {
                        return Err(Self::policy_err(
                            "tools is only allowed in @system",
                            "tools",
                        ));
                    }
                    if !matches!(kv.value, ValueNode::List(_)) {
                        return Err(Self::policy_err("@system.tools must be a list", "tools"));
                    }
                }
                _ => {}
            }

            Self::validate_value_map_keys(&kv.value, false)?;
        }

        if !content_seen {
            return Err(Self::policy_err(
                "Message block must contain required content field",
                &block.name,
            ));
        }

        Ok(())
    }

    fn validate_message_content(&self, value: &ValueNode) -> ValidationResult<()> {
        match value {
            ValueNode::String(_) | ValueNode::Variable(_) | ValueNode::Pipeline(_) => Ok(()),
            ValueNode::List(items) => {
                for item in items {
                    let map = match item {
                        ValueNode::Map(map) => map,
                        _ => {
                            return Err(Self::policy_err(
                                "Message content list items must be maps",
                                "content",
                            ))
                        }
                    };

                    let item_type = match map.get("type") {
                        Some(ValueNode::String(t)) => t.as_str(),
                        _ => {
                            return Err(Self::policy_err(
                                "Content item requires string 'type'",
                                "type",
                            ))
                        }
                    };

                    match item_type {
                        "text" => match map.get("text") {
                            Some(ValueNode::String(_)) => {}
                            _ => {
                                return Err(Self::policy_err(
                                    "Text content item requires string 'text'",
                                    "text",
                                ))
                            }
                        },
                        "image" | "audio" => self.validate_canonical_asset(item_type, map)?,
                        _ => {
                            return Err(Self::policy_err(
                                "Unsupported content item type",
                                item_type,
                            ))
                        }
                    }

                    Self::validate_value_map_keys(item, false)?;
                }
                Ok(())
            }
            _ => Err(Self::policy_err(
                "Message content must be string or list of content items",
                "content",
            )),
        }
    }

    fn validate_canonical_asset(
        &self,
        item_type: &str,
        item_map: &OrderedMap<String, ValueNode>,
    ) -> ValidationResult<()> {
        let asset_map = match item_map.get("asset") {
            Some(ValueNode::Map(map)) => map,
            _ => {
                return Err(Self::policy_err(
                    "Image/audio content item requires map 'asset'",
                    "asset",
                ))
            }
        };

        let kind = match asset_map.get("kind") {
            Some(ValueNode::String(v)) => v,
            _ => {
                return Err(Self::policy_err(
                    "Asset requires string 'kind'",
                    "asset.kind",
                ))
            }
        };
        if kind != item_type {
            return Err(Self::policy_err(
                "Asset kind must match content item type",
                "asset.kind",
            ));
        }

        let format = match asset_map.get("format") {
            Some(ValueNode::String(v)) => v.as_str(),
            _ => {
                return Err(Self::policy_err(
                    "Asset requires string 'format'",
                    "asset.format",
                ))
            }
        };

        match item_type {
            "image" if !matches!(format, "png" | "jpeg" | "webp") => {
                return Err(Self::policy_err(
                    "Image asset format must be one of: png|jpeg|webp",
                    "asset.format",
                ));
            }
            "audio" if !matches!(format, "mp3" | "wav" | "ogg") => {
                return Err(Self::policy_err(
                    "Audio asset format must be one of: mp3|wav|ogg",
                    "asset.format",
                ));
            }
            _ => {}
        }

        let digest = match asset_map.get("digest") {
            Some(ValueNode::Map(map)) => map,
            _ => {
                return Err(Self::policy_err(
                    "Asset requires map 'digest'",
                    "asset.digest",
                ))
            }
        };
        match digest.get("algo") {
            Some(ValueNode::String(v)) if v == "sha256" => {}
            _ => {
                return Err(Self::policy_err(
                    "Asset digest.algo must be string 'sha256'",
                    "asset.digest.algo",
                ))
            }
        }
        match digest.get("value") {
            Some(ValueNode::String(_)) => {}
            _ => {
                return Err(Self::policy_err(
                    "Asset digest.value must be string",
                    "asset.digest.value",
                ))
            }
        }

        let shape = match asset_map.get("shape") {
            Some(ValueNode::Map(map)) => map,
            _ => {
                return Err(Self::policy_err(
                    "Asset requires map 'shape'",
                    "asset.shape",
                ))
            }
        };

        match item_type {
            "image" => {
                match shape.get("width") {
                    Some(ValueNode::Scalar(ScalarValue::Int(v))) if *v >= 0 => {}
                    _ => {
                        return Err(Self::policy_err(
                            "Image asset shape.width must be int >= 0",
                            "asset.shape.width",
                        ))
                    }
                }
                match shape.get("height") {
                    Some(ValueNode::Scalar(ScalarValue::Int(v))) if *v >= 0 => {}
                    _ => {
                        return Err(Self::policy_err(
                            "Image asset shape.height must be int >= 0",
                            "asset.shape.height",
                        ))
                    }
                }
            }
            "audio" => match shape.get("duration") {
                Some(ValueNode::Scalar(ScalarValue::Int(v))) if *v >= 0 => {}
                Some(ValueNode::Scalar(ScalarValue::Float(v))) if *v >= 0.0 => {}
                _ => {
                    return Err(Self::policy_err(
                        "Audio asset shape.duration must be number >= 0",
                        "asset.shape.duration",
                    ))
                }
            },
            _ => {}
        }

        Ok(())
    }

    fn validate_when_atom(&self, value: &ValueNode) -> ValidationResult<()> {
        match value {
            ValueNode::Scalar(ScalarValue::Bool(_)) => Ok(()),
            ValueNode::Variable(var_ref) => {
                let base = var_ref.split('.').next().unwrap_or(var_ref);
                let declared_type = self
                    .variables
                    .get(base)
                    .cloned()
                    .or_else(|| self.var_types.get(base).map(|decl| decl.var_type.clone()))
                    .ok_or_else(|| ValidationError::VariableNotFound {
                        var: base.to_string(),
                    })?;

                let is_bool_like = matches!(
                    declared_type,
                    FacetType::Primitive(crate::types::PrimitiveType::Bool)
                        | FacetType::Primitive(crate::types::PrimitiveType::Any)
                );
                if is_bool_like {
                    Ok(())
                } else {
                    Err(ValidationError::TypeMismatch {
                        expected: "Primitive(Bool)".to_string(),
                        got: format!("{:?}", declared_type),
                        location: "message when".to_string(),
                    })
                }
            }
            _ => Err(ValidationError::TypeMismatch {
                expected: "Primitive(Bool)".to_string(),
                got: format!("{:?}", self.infer_type(value)?),
                location: "message when".to_string(),
            }),
        }
    }

    fn validate_input_usage_for_var(&self, value: &ValueNode, name: &str) -> ValidationResult<()> {
        if !Self::contains_input_directive(value) {
            return Ok(());
        }

        match value {
            ValueNode::Directive(directive) if directive.name == "input" => {
                self.validate_input_directive(directive, name)
            }
            ValueNode::Pipeline(pipeline) => {
                let base = pipeline.initial.as_ref();
                let directive = match base {
                    ValueNode::Directive(d) if d.name == "input" => d,
                    _ => {
                        return Err(Self::policy_err(
                            "@input(...) must be the base expression of an @vars entry value",
                            name,
                        ))
                    }
                };

                if pipeline
                    .lenses
                    .iter()
                    .flat_map(|lens| lens.args.iter().chain(lens.kwargs.values()))
                    .any(Self::contains_input_directive)
                {
                    return Err(Self::policy_err(
                        "@input(...) is not allowed inside lens arguments",
                        name,
                    ));
                }

                self.validate_input_directive(directive, name)
            }
            _ => Err(Self::policy_err(
                "@input(...) must appear only as direct value or pipeline base in @vars",
                name,
            )),
        }
    }

    fn validate_input_directive(
        &self,
        directive: &fct_ast::DirectiveNode,
        var_name: &str,
    ) -> ValidationResult<()> {
        if directive.name != "input" {
            return Err(Self::policy_err(
                "Unsupported directive in @vars",
                &directive.name,
            ));
        }

        let input_type = directive
            .args
            .get("type")
            .ok_or_else(|| Self::policy_err("@input(...) requires 'type' attribute", var_name))?;
        let input_type_str = match input_type {
            ValueNode::String(s) => s,
            _ => {
                return Err(Self::policy_err(
                    "@input(type=...) must be a string",
                    var_name,
                ))
            }
        };

        if self.parse_type_string(input_type_str).is_err() {
            return Err(Self::policy_err(
                "Invalid @input type expression",
                input_type_str,
            ));
        }

        if let Some(default_value) = directive.args.get("default") {
            if !Self::is_atom_value(default_value) {
                return Err(Self::policy_err("@input default must be an atom", var_name));
            }
        }

        Ok(())
    }

    fn validate_value_map_keys(value: &ValueNode, allow_string_keys: bool) -> ValidationResult<()> {
        match value {
            ValueNode::Map(map) => {
                for (key, nested) in map {
                    if !allow_string_keys && !Self::is_identifier(key) {
                        return Err(Self::policy_err(
                            "String-keyed maps are only allowed in @meta",
                            key,
                        ));
                    }
                    Self::validate_value_map_keys(nested, allow_string_keys)?;
                }
            }
            ValueNode::List(items) => {
                for item in items {
                    Self::validate_value_map_keys(item, allow_string_keys)?;
                }
            }
            ValueNode::Pipeline(p) => {
                Self::validate_value_map_keys(&p.initial, allow_string_keys)?;
                for lens in &p.lenses {
                    for arg in &lens.args {
                        Self::validate_value_map_keys(arg, allow_string_keys)?;
                    }
                    for arg in lens.kwargs.values() {
                        Self::validate_value_map_keys(arg, allow_string_keys)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn ensure_identifier_block_key(&self, kv: &KeyValueNode) -> ValidationResult<()> {
        if kv.key_kind == MapKeyKind::String {
            return Err(Self::policy_err(
                "String-keyed maps are only allowed in @meta",
                &kv.key,
            ));
        }
        Ok(())
    }

    fn is_atom_value(value: &ValueNode) -> bool {
        matches!(value, ValueNode::String(_) | ValueNode::Scalar(_))
    }

    fn contains_input_directive(value: &ValueNode) -> bool {
        match value {
            ValueNode::Directive(d) => d.name == "input",
            ValueNode::List(items) => items.iter().any(Self::contains_input_directive),
            ValueNode::Map(map) => map.values().any(Self::contains_input_directive),
            ValueNode::Pipeline(p) => {
                Self::contains_input_directive(&p.initial)
                    || p.lenses
                        .iter()
                        .flat_map(|lens| lens.args.iter().chain(lens.kwargs.values()))
                        .any(Self::contains_input_directive)
            }
            _ => false,
        }
    }

    fn contains_control_chars(s: &str) -> bool {
        s.chars().any(|c| (c as u32) <= 0x1F || (c as u32) == 0x7F)
    }

    fn validate_policy(&self, doc: &FacetDocument) -> ValidationResult<()> {
        let known_tool_functions = self.collect_interface_function_index(doc);
        let known_lenses: HashSet<String> = self._lens_provider.lens_names().into_iter().collect();
        let known_message_ids = self.collect_message_ids(doc);
        for block in &doc.blocks {
            if let FacetNode::Policy(policy_block) = block {
                self.validate_policy_block(
                    policy_block,
                    &known_tool_functions,
                    &known_lenses,
                    &known_message_ids,
                )?;
            }
        }
        Ok(())
    }

    fn validate_policy_block(
        &self,
        block: &FacetBlock,
        known_tool_functions: &HashMap<String, HashSet<String>>,
        known_lenses: &HashSet<String>,
        known_message_ids: &HashSet<String>,
    ) -> ValidationResult<()> {
        for body_node in &block.body {
            let kv = match body_node {
                BodyNode::KeyValue(kv) => kv,
                _ => {
                    return Err(Self::policy_err(
                        "Invalid @policy body entry (expected key:value)",
                        "@policy",
                    ))
                }
            };

            match kv.key.as_str() {
                "defaults" => {
                    if !matches!(kv.value, ValueNode::Map(_)) {
                        return Err(Self::policy_err(
                            "@policy.defaults must be a map",
                            "defaults",
                        ));
                    }
                    self.validate_policy_defaults(&kv.value)?;
                }
                "allow" | "deny" => self.validate_policy_rule_list(
                    &kv.value,
                    &kv.key,
                    known_tool_functions,
                    known_lenses,
                    known_message_ids,
                )?,
                _ => return Err(Self::policy_err("Unknown @policy top-level key", &kv.key)),
            }
        }

        Ok(())
    }

    fn validate_policy_defaults(&self, node: &ValueNode) -> ValidationResult<()> {
        let ValueNode::Map(defaults) = node else {
            return Err(Self::policy_err(
                "@policy.defaults must be a map",
                "defaults",
            ));
        };

        for (op, value) in defaults {
            if !matches!(
                op.as_str(),
                "tool_expose" | "tool_call" | "lens_call" | "message_emit"
            ) {
                return Err(Self::policy_err(
                    "Unknown @policy.defaults operation key",
                    op,
                ));
            }

            match value {
                ValueNode::String(v) if v == "allow" || v == "deny" => {}
                ValueNode::Scalar(ScalarValue::Bool(_)) => {}
                _ => {
                    return Err(Self::policy_err(
                        "@policy.defaults values must be bool or string 'allow'|'deny'",
                        op,
                    ))
                }
            }
        }

        Ok(())
    }

    fn validate_policy_rule_list(
        &self,
        node: &ValueNode,
        list_name: &str,
        known_tool_functions: &HashMap<String, HashSet<String>>,
        known_lenses: &HashSet<String>,
        known_message_ids: &HashSet<String>,
    ) -> ValidationResult<()> {
        let rules = match node {
            ValueNode::List(items) => items,
            _ => {
                return Err(Self::policy_err(
                    &format!("@policy.{} must be a list", list_name),
                    list_name,
                ))
            }
        };

        for rule in rules {
            self.validate_policy_rule(rule, known_tool_functions, known_lenses, known_message_ids)?;
        }
        Ok(())
    }

    fn validate_policy_rule(
        &self,
        rule: &ValueNode,
        known_tool_functions: &HashMap<String, HashSet<String>>,
        known_lenses: &HashSet<String>,
        known_message_ids: &HashSet<String>,
    ) -> ValidationResult<()> {
        let map = match rule {
            ValueNode::Map(map) => map,
            _ => return Err(Self::policy_err("PolicyRule must be a map", "rule")),
        };

        for key in map.keys() {
            match key.as_str() {
                "id" | "op" | "name" | "effect" | "when" | "unless" => {}
                _ => return Err(Self::policy_err("Unknown key inside PolicyRule", key)),
            }
        }

        if let Some(id_val) = map.get("id") {
            if !matches!(id_val, ValueNode::String(_)) {
                return Err(Self::policy_err("PolicyRule.id must be string", "id"));
            }
        }

        let op = match map.get("op") {
            Some(ValueNode::String(op)) => op.as_str(),
            _ => {
                return Err(Self::policy_err(
                    "PolicyRule.op is required and must be string",
                    "op",
                ))
            }
        };

        match op {
            "tool_expose" | "tool_call" | "lens_call" | "message_emit" => {}
            _ => return Err(Self::policy_err("PolicyRule.op has unsupported value", op)),
        }

        let name_required = matches!(op, "tool_expose" | "tool_call" | "lens_call");
        if name_required && !map.contains_key("name") {
            return Err(Self::policy_err(
                "PolicyRule.name is required for this op",
                op,
            ));
        }

        if let Some(name_val) = map.get("name") {
            let name = match name_val {
                ValueNode::String(v) => v,
                _ => return Err(Self::policy_err("PolicyRule.name must be string", "name")),
            };
            self.validate_name_matcher(
                name,
                op,
                known_tool_functions,
                known_lenses,
                known_message_ids,
            )?;
        }

        if let Some(effect_val) = map.get("effect") {
            let effect = match effect_val {
                ValueNode::String(v) => v,
                _ => {
                    return Err(Self::policy_err(
                        "PolicyRule.effect must be string",
                        "effect",
                    ))
                }
            };
            self.validate_effect_matcher(effect)?;
        }

        if let Some(when_val) = map.get("when") {
            self.validate_policy_cond(when_val)?;
        }
        if let Some(unless_val) = map.get("unless") {
            self.validate_policy_cond(unless_val)?;
        }

        Ok(())
    }

    fn validate_name_matcher(
        &self,
        name: &str,
        op: &str,
        known_tool_functions: &HashMap<String, HashSet<String>>,
        known_lenses: &HashSet<String>,
        known_message_ids: &HashSet<String>,
    ) -> ValidationResult<()> {
        if name.chars().any(char::is_whitespace) {
            return Err(Self::policy_err(
                "PolicyRule.name must not contain whitespace",
                name,
            ));
        }
        self.validate_matcher_wildcard(name, "name")?;

        match op {
            "tool_expose" | "tool_call" => {
                if name.ends_with(".*") {
                    let iface = &name[..name.len().saturating_sub(2)];
                    if !Self::is_identifier(iface) {
                        return Err(Self::policy_err(
                            "Tool name wildcard matcher must be <InterfaceName>.*",
                            name,
                        ));
                    }
                    if !known_tool_functions.contains_key(iface) {
                        return Err(Self::policy_err(
                            "Unknown interface in PolicyRule.name matcher",
                            name,
                        ));
                    }
                } else {
                    let mut parts = name.split('.');
                    let iface = parts.next().unwrap_or_default();
                    let func = parts.next().unwrap_or_default();
                    if parts.next().is_some()
                        || !Self::is_identifier(iface)
                        || !Self::is_identifier(func)
                    {
                        return Err(Self::policy_err(
                            "Tool name must be canonical <InterfaceName>.<fn_name>",
                            name,
                        ));
                    }
                    let known_funcs = known_tool_functions.get(iface).ok_or_else(|| {
                        Self::policy_err("Unknown interface in PolicyRule.name", name)
                    })?;
                    if !known_funcs.contains(func) {
                        return Err(Self::policy_err(
                            "Unknown interface function in PolicyRule.name",
                            name,
                        ));
                    }
                }
            }
            "lens_call" => {
                let candidate = if name.ends_with(".*") {
                    &name[..name.len().saturating_sub(2)]
                } else {
                    name
                };
                if candidate.is_empty() {
                    return Err(Self::policy_err("Lens matcher cannot be empty", name));
                }
                if !candidate
                    .split('.')
                    .all(|segment| Self::is_identifier(segment))
                {
                    return Err(Self::policy_err(
                        "Lens name must be canonical identifier(.identifier)*",
                        name,
                    ));
                }

                if name.ends_with(".*") {
                    if !known_lenses.iter().any(|lens| lens.starts_with(candidate)) {
                        return Err(Self::policy_err(
                            "Unknown lens matcher prefix in PolicyRule.name",
                            name,
                        ));
                    }
                } else if !known_lenses.contains(candidate) {
                    return Err(Self::policy_err("Unknown lens in PolicyRule.name", name));
                }
            }
            "message_emit" => {
                let candidate = if name.ends_with(".*") {
                    &name[..name.len().saturating_sub(2)]
                } else {
                    name
                };
                if candidate.is_empty() {
                    return Err(Self::policy_err(
                        "message_emit name matcher cannot be empty",
                        name,
                    ));
                }

                if name.ends_with(".*") {
                    if !known_message_ids.iter().any(|id| id.starts_with(candidate)) {
                        return Err(Self::policy_err(
                            "Unknown message id matcher prefix in PolicyRule.name",
                            name,
                        ));
                    }
                } else if !known_message_ids.contains(candidate) {
                    return Err(Self::policy_err(
                        "Unknown message id in PolicyRule.name",
                        name,
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn collect_interface_function_index(
        &self,
        doc: &FacetDocument,
    ) -> HashMap<String, HashSet<String>> {
        let mut index: HashMap<String, HashSet<String>> = HashMap::new();
        for node in &doc.blocks {
            if let FacetNode::Interface(interface) = node {
                let entry = index.entry(interface.name.clone()).or_default();
                for func in &interface.functions {
                    entry.insert(func.name.clone());
                }
            }
        }
        index
    }

    fn collect_message_ids(&self, doc: &FacetDocument) -> HashSet<String> {
        let mut ids = HashSet::new();

        for role in ["system", "user", "assistant"] {
            let mut n = 0usize;
            for node in &doc.blocks {
                let block = match (role, node) {
                    ("system", FacetNode::System(block)) => Some(block),
                    ("user", FacetNode::User(block)) => Some(block),
                    ("assistant", FacetNode::Assistant(block)) => Some(block),
                    _ => None,
                };
                let Some(block) = block else {
                    continue;
                };

                n += 1;
                let id = explicit_message_id(block).unwrap_or_else(|| format!("{role}#{n}"));
                ids.insert(id);
            }
        }

        ids
    }

    fn validate_effect_matcher(&self, effect: &str) -> ValidationResult<()> {
        if effect.chars().any(char::is_whitespace) {
            return Err(Self::policy_err(
                "PolicyRule.effect must not contain whitespace",
                effect,
            ));
        }
        self.validate_matcher_wildcard(effect, "effect")
    }

    fn validate_matcher_wildcard(&self, s: &str, field: &str) -> ValidationResult<()> {
        if s.contains('*') && !s.ends_with(".*") {
            return Err(Self::policy_err(
                &format!(
                    "PolicyRule.{} wildcard is only allowed as suffix '.*'",
                    field
                ),
                s,
            ));
        }
        if s.matches('*').count() > 1 {
            return Err(Self::policy_err(
                &format!("PolicyRule.{} wildcard is invalid", field),
                s,
            ));
        }
        Ok(())
    }

    fn validate_policy_cond(&self, cond: &ValueNode) -> ValidationResult<()> {
        match cond {
            ValueNode::Scalar(ScalarValue::Bool(_)) => Ok(()),
            ValueNode::Variable(var_ref) => {
                let base = var_ref.split('.').next().unwrap_or(var_ref);
                let declared_type = self
                    .variables
                    .get(base)
                    .cloned()
                    .or_else(|| self.var_types.get(base).map(|decl| decl.var_type.clone()));

                if declared_type.is_none() {
                    return Err(ValidationError::VariableNotFound {
                        var: base.to_string(),
                    });
                }

                if let Some(var_type) = declared_type {
                    let is_bool_like = matches!(
                        var_type,
                        FacetType::Primitive(crate::types::PrimitiveType::Bool)
                            | FacetType::Primitive(crate::types::PrimitiveType::Any)
                    );
                    if !is_bool_like {
                        return Err(ValidationError::TypeMismatch {
                            expected: "Primitive(Bool)".to_string(),
                            got: format!("{:?}", var_type),
                            location: "policy condition".to_string(),
                        });
                    }
                }

                if var_ref
                    .split('.')
                    .skip(1)
                    .any(|seg| seg.chars().all(|c| c.is_ascii_digit()))
                {
                    return Err(Self::policy_err(
                        "PolicyCond numeric path indexing is not supported",
                        var_ref,
                    ));
                }
                Ok(())
            }
            ValueNode::Map(map) => {
                if map.len() != 1 {
                    return Err(Self::policy_err(
                        "PolicyCond map form must contain exactly one operator",
                        "cond",
                    ));
                }
                let (op, value) = map.iter().next().expect("len checked");
                match op.as_str() {
                    "not" => self.validate_policy_cond(value),
                    "all" | "any" => {
                        let items = match value {
                            ValueNode::List(items) => items,
                            _ => {
                                return Err(Self::policy_err(
                                    "PolicyCond all/any expects a list",
                                    op,
                                ))
                            }
                        };
                        if items.is_empty() {
                            return Err(Self::policy_err(
                                "PolicyCond all/any list must be non-empty",
                                op,
                            ));
                        }
                        for item in items {
                            self.validate_policy_cond(item)?;
                        }
                        Ok(())
                    }
                    _ => Err(Self::policy_err("Unknown PolicyCond operator", op)),
                }
            }
            ValueNode::Pipeline(_) | ValueNode::Directive(_) => Err(Self::policy_err(
                "PolicyCond must not contain pipelines or directives",
                "cond",
            )),
            _ => Err(Self::policy_err(
                "PolicyCond must be bool literal, varref, or map form",
                "cond",
            )),
        }
    }

    fn validate_interface_type_mappable(ty: &TypeNode, location: &str) -> ValidationResult<()> {
        match ty {
            TypeNode::Primitive(name) => match name.as_str() {
                "string" | "int" | "float" | "bool" | "null" | "any" => Ok(()),
                _ => Err(Self::policy_err(
                    "Interface type is not mappable to JSON Schema (Appendix D)",
                    location,
                )),
            },
            TypeNode::Struct(fields) => {
                for field_ty in fields.values() {
                    Self::validate_interface_type_mappable(field_ty, location)?;
                }
                Ok(())
            }
            TypeNode::List(item_ty) | TypeNode::Map(item_ty) => {
                Self::validate_interface_type_mappable(item_ty, location)
            }
            TypeNode::Union(types) => {
                if types.is_empty() {
                    return Err(Self::policy_err(
                        "Union type must contain at least one member",
                        location,
                    ));
                }
                for member in types {
                    Self::validate_interface_type_mappable(member, location)?;
                }
                Ok(())
            }
            TypeNode::Embedding { size } => {
                if *size == 0 {
                    return Err(Self::policy_err(
                        "Embedding size must be a positive integer",
                        location,
                    ));
                }
                Ok(())
            }
            // Appendix D does not define JSON Schema mapping for these forms.
            TypeNode::Image { .. } | TypeNode::Audio { .. } => Err(Self::policy_err(
                "Interface type is not mappable to JSON Schema (Appendix D)",
                location,
            )),
        }
    }

    fn policy_err(constraint: &str, value: &str) -> ValidationError {
        ValidationError::ConstraintViolation {
            constraint: constraint.to_string(),
            value: value.to_string(),
        }
    }

    fn is_identifier(s: &str) -> bool {
        let mut chars = s.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !(first.is_ascii_alphabetic() || first == '_') {
            return false;
        }
        chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    fn is_valid_effect_class(effect: &str) -> bool {
        matches!(
            effect,
            "read" | "write" | "external" | "payment" | "filesystem" | "network"
        ) || {
            let mut parts = effect.split('.');
            matches!(parts.next(), Some("x"))
                && parts.next().map(Self::is_identifier).unwrap_or(false)
                && parts.next().map(Self::is_identifier).unwrap_or(false)
                && parts.next().is_none()
        }
    }
}

fn explicit_message_id(block: &FacetBlock) -> Option<String> {
    for entry in &block.body {
        if let BodyNode::KeyValue(kv) = entry {
            if kv.key == "id" {
                if let ValueNode::String(id) = &kv.value {
                    return Some(id.clone());
                }
            }
        }
    }
    None
}

fn value_matches_expected_type<S: LensSignatureProvider>(
    value: &ValueNode,
    expected_type: &FacetType,
    checker: &TypeChecker<S>,
) -> ValidationResult<bool> {
    if matches!(
        expected_type,
        FacetType::Primitive(crate::types::PrimitiveType::Any)
    ) {
        return Ok(true);
    }

    if let FacetType::Union(union_members) = expected_type {
        for member in union_members {
            if value_matches_expected_type(value, member, checker)? {
                return Ok(true);
            }
        }
        return Ok(false);
    }

    match value {
        ValueNode::String(_) => Ok(matches!(
            expected_type,
            FacetType::Primitive(crate::types::PrimitiveType::String)
        )),
        ValueNode::Scalar(ScalarValue::Int(_)) => Ok(matches!(
            expected_type,
            FacetType::Primitive(crate::types::PrimitiveType::Int)
        )),
        ValueNode::Scalar(ScalarValue::Float(_)) => Ok(matches!(
            expected_type,
            FacetType::Primitive(crate::types::PrimitiveType::Float)
        )),
        ValueNode::Scalar(ScalarValue::Bool(_)) => Ok(matches!(
            expected_type,
            FacetType::Primitive(crate::types::PrimitiveType::Bool)
        )),
        ValueNode::Scalar(ScalarValue::Null) => Ok(matches!(
            expected_type,
            FacetType::Primitive(crate::types::PrimitiveType::Null)
        )),
        ValueNode::List(items) => match expected_type {
            FacetType::List(list_ty) => {
                for item in items {
                    if !value_matches_expected_type(item, list_ty.as_ref(), checker)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            FacetType::Multimodal(crate::types::MultimodalType::Embedding(embed_ty)) => {
                if items.len() != embed_ty.size {
                    return Ok(false);
                }
                Ok(items.iter().all(|item| {
                    matches!(
                        item,
                        ValueNode::Scalar(ScalarValue::Int(_))
                            | ValueNode::Scalar(ScalarValue::Float(_))
                    )
                }))
            }
            _ => Ok(false),
        },
        ValueNode::Map(map) => match expected_type {
            FacetType::Struct(struct_ty) => {
                for field in struct_ty {
                    if let Some(field_value) = map.get(&field.name) {
                        if !value_matches_expected_type(field_value, &field.field_type, checker)? {
                            return Ok(false);
                        }
                    } else if field.required {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            FacetType::Map(map_ty) => {
                for map_value in map.values() {
                    if !value_matches_expected_type(map_value, map_ty.as_ref(), checker)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            FacetType::Multimodal(crate::types::MultimodalType::Image(image_ty)) => {
                if !matches!(map.get("kind"), Some(ValueNode::String(kind)) if kind == "image") {
                    return Ok(false);
                }
                if let Some(required_format) = &image_ty.format {
                    if !matches!(map.get("format"), Some(ValueNode::String(v)) if v == required_format)
                    {
                        return Ok(false);
                    }
                }
                if let Some(max_dim) = image_ty.max_dim {
                    let Some(ValueNode::Map(shape)) = map.get("shape") else {
                        return Ok(false);
                    };
                    let width_ok = matches!(
                        shape.get("width"),
                        Some(ValueNode::Scalar(ScalarValue::Int(width))) if *width >= 0 && (*width as u32) <= max_dim
                    );
                    let height_ok = matches!(
                        shape.get("height"),
                        Some(ValueNode::Scalar(ScalarValue::Int(height))) if *height >= 0 && (*height as u32) <= max_dim
                    );
                    if !(width_ok && height_ok) {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            FacetType::Multimodal(crate::types::MultimodalType::Audio(audio_ty)) => {
                if !matches!(map.get("kind"), Some(ValueNode::String(kind)) if kind == "audio") {
                    return Ok(false);
                }
                if let Some(required_format) = &audio_ty.format {
                    if !matches!(map.get("format"), Some(ValueNode::String(v)) if v == required_format)
                    {
                        return Ok(false);
                    }
                }
                if let Some(max_duration) = audio_ty.max_duration {
                    let Some(ValueNode::Map(shape)) = map.get("shape") else {
                        return Ok(false);
                    };
                    let duration = match shape.get("duration") {
                        Some(ValueNode::Scalar(ScalarValue::Int(v))) => *v as f64,
                        Some(ValueNode::Scalar(ScalarValue::Float(v))) => *v,
                        _ => return Ok(false),
                    };
                    if duration < 0.0 || duration > max_duration {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        },
        ValueNode::Variable(var_ref) => {
            let Some(actual_type) = resolve_variable_type(checker, var_ref) else {
                // Forward refs are valid in @vars and resolved in Phase 3.
                return Ok(true);
            };
            Ok(actual_type.is_assignable_to(expected_type))
        }
        ValueNode::Pipeline(_) => {
            let actual_type = checker.infer_pipeline_checked_type(value, "variable assignment")?;
            Ok(actual_type.is_assignable_to(expected_type))
        }
        ValueNode::Directive(directive) => {
            if directive.name == "input" {
                if let Some(ValueNode::String(type_str)) = directive.args.get("type") {
                    let actual_type = parse_type_expr(type_str)?;
                    return Ok(actual_type.is_assignable_to(expected_type));
                }
            }
            Ok(false)
        }
    }
}

fn resolve_variable_type<S: LensSignatureProvider>(
    checker: &TypeChecker<S>,
    var_ref: &str,
) -> Option<FacetType> {
    let mut segments = var_ref.split('.');
    let base = segments.next()?;
    let mut current = checker.variables.get(base).cloned().or_else(|| {
        checker
            .var_types
            .get(base)
            .map(|decl| decl.var_type.clone())
    })?;

    for segment in segments {
        current = match current {
            FacetType::Struct(struct_ty) => struct_field_type(&struct_ty, segment)?,
            FacetType::Map(map_ty) => (*map_ty).clone(),
            FacetType::Union(union_ty) => {
                let mut next_variants = Vec::new();
                for variant in union_ty {
                    match variant {
                        FacetType::Struct(struct_ty) => {
                            if let Some(next_ty) = struct_field_type(&struct_ty, segment) {
                                next_variants.push(next_ty);
                            }
                        }
                        FacetType::Map(map_ty) => next_variants.push((*map_ty).clone()),
                        FacetType::Primitive(crate::types::PrimitiveType::Any) => {
                            return Some(FacetType::Primitive(crate::types::PrimitiveType::Any));
                        }
                        _ => {}
                    }
                }
                if next_variants.is_empty() {
                    return None;
                }
                if next_variants.len() == 1 {
                    next_variants.remove(0)
                } else {
                    FacetType::Union(next_variants)
                }
            }
            FacetType::Primitive(crate::types::PrimitiveType::Any) => {
                FacetType::Primitive(crate::types::PrimitiveType::Any)
            }
            _ => return None,
        };
    }

    Some(current)
}

fn struct_field_type(struct_fields: &[crate::types::StructField], name: &str) -> Option<FacetType> {
    struct_fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.field_type.clone())
}

fn parse_type_expr(type_str: &str) -> ValidationResult<FacetType> {
    let mut parser = TypeExprParser::new(type_str);
    let parsed = parser.parse_type_expr()?;
    parser.skip_ws();
    if parser.is_eof() {
        Ok(parsed)
    } else {
        Err(parser.err("Unexpected trailing characters in type expression"))
    }
}

struct TypeExprParser<'a> {
    input: &'a str,
    offset: usize,
}

impl<'a> TypeExprParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, offset: 0 }
    }

    fn parse_type_expr(&mut self) -> ValidationResult<FacetType> {
        self.parse_union_type()
    }

    fn parse_union_type(&mut self) -> ValidationResult<FacetType> {
        let first = self.parse_primary_type()?;
        let mut members = vec![first];

        loop {
            self.skip_ws();
            if !self.consume_char('|') {
                break;
            }
            self.skip_ws();
            members.push(self.parse_primary_type()?);
        }

        if members.len() == 1 {
            Ok(members.remove(0))
        } else {
            Ok(FacetType::Union(members))
        }
    }

    fn parse_primary_type(&mut self) -> ValidationResult<FacetType> {
        self.skip_ws();
        if self.starts_keyword("struct") {
            return self.parse_struct_type();
        }
        if self.starts_keyword("list") {
            return self.parse_list_type();
        }
        if self.starts_keyword("map") {
            return self.parse_map_type();
        }
        if self.starts_keyword("embedding") {
            return self.parse_embedding_type();
        }
        if self.starts_keyword("image") {
            return self.parse_image_type();
        }
        if self.starts_keyword("audio") {
            return self.parse_audio_type();
        }

        let ident = self.parse_identifier()?;
        match ident.as_str() {
            "string" => Ok(FacetType::Primitive(crate::types::PrimitiveType::String)),
            "int" => Ok(FacetType::Primitive(crate::types::PrimitiveType::Int)),
            "float" => Ok(FacetType::Primitive(crate::types::PrimitiveType::Float)),
            "bool" => Ok(FacetType::Primitive(crate::types::PrimitiveType::Bool)),
            "null" => Ok(FacetType::Primitive(crate::types::PrimitiveType::Null)),
            "any" => Ok(FacetType::Primitive(crate::types::PrimitiveType::Any)),
            _ => Err(self.err("Unknown type identifier")),
        }
    }

    fn parse_list_type(&mut self) -> ValidationResult<FacetType> {
        self.expect_keyword("list")?;
        self.skip_ws();
        self.expect_char('<')?;
        let item_type = self.parse_type_expr()?;
        self.skip_ws();
        self.expect_char('>')?;
        Ok(FacetType::List(Box::new(item_type)))
    }

    fn parse_map_type(&mut self) -> ValidationResult<FacetType> {
        self.expect_keyword("map")?;
        self.skip_ws();
        self.expect_char('<')?;
        self.skip_ws();

        let key_type = self.parse_identifier()?;
        if key_type != "string" && key_type != "String" {
            return Err(self.err("map key type must be string"));
        }

        self.skip_ws();
        self.expect_char(',')?;
        let value_type = self.parse_type_expr()?;
        self.skip_ws();
        self.expect_char('>')?;

        Ok(FacetType::Map(Box::new(value_type)))
    }

    fn parse_struct_type(&mut self) -> ValidationResult<FacetType> {
        self.expect_keyword("struct")?;
        self.skip_ws();
        self.expect_char('{')?;
        let mut fields: Vec<crate::types::StructField> = Vec::new();

        loop {
            self.skip_ws();
            if self.consume_char('}') {
                break;
            }

            let field_name = self.parse_identifier()?;
            self.skip_ws();
            self.expect_char(':')?;
            let field_type = self.parse_type_expr()?;
            let next_field = crate::types::StructField {
                name: field_name,
                field_type,
                required: true,
            };
            if let Some(existing) = fields
                .iter_mut()
                .find(|field| field.name == next_field.name)
            {
                *existing = next_field;
            } else {
                fields.push(next_field);
            }

            self.skip_ws();
            if self.consume_char(',') {
                continue;
            }
            if self.consume_char('}') {
                break;
            }
            return Err(self.err("Expected ',' or '}' in struct type"));
        }

        Ok(FacetType::Struct(fields))
    }

    fn parse_embedding_type(&mut self) -> ValidationResult<FacetType> {
        self.expect_keyword("embedding")?;
        self.skip_ws();
        self.expect_char('<')?;
        self.skip_ws();
        self.expect_keyword("size")?;
        self.skip_ws();
        self.expect_char('=')?;
        self.skip_ws();
        let size = self.parse_usize()?;
        if size == 0 {
            return Err(self.err("embedding size must be positive"));
        }
        self.skip_ws();
        self.expect_char('>')?;
        Ok(FacetType::Multimodal(
            crate::types::MultimodalType::Embedding(crate::types::EmbeddingType { size }),
        ))
    }

    fn parse_image_type(&mut self) -> ValidationResult<FacetType> {
        self.expect_keyword("image")?;
        let (format, max_dim, _) = self.parse_media_constraints(true)?;
        if let Some(ref fmt) = format {
            if !matches!(fmt.as_str(), "png" | "jpeg" | "webp") {
                return Err(self.err("image format must be one of png|jpeg|webp"));
            }
        }
        Ok(FacetType::Multimodal(crate::types::MultimodalType::Image(
            crate::types::ImageType { max_dim, format },
        )))
    }

    fn parse_audio_type(&mut self) -> ValidationResult<FacetType> {
        self.expect_keyword("audio")?;
        let (format, _, max_duration) = self.parse_media_constraints(false)?;
        if let Some(ref fmt) = format {
            if !matches!(fmt.as_str(), "mp3" | "wav" | "ogg") {
                return Err(self.err("audio format must be one of mp3|wav|ogg"));
            }
        }
        Ok(FacetType::Multimodal(crate::types::MultimodalType::Audio(
            crate::types::AudioType {
                max_duration,
                format,
            },
        )))
    }

    fn parse_media_constraints(
        &mut self,
        image: bool,
    ) -> ValidationResult<(Option<String>, Option<u32>, Option<f64>)> {
        self.skip_ws();
        if !self.consume_char('(') {
            return Ok((None, None, None));
        }

        let mut format: Option<String> = None;
        let mut max_dim: Option<u32> = None;
        let mut max_duration: Option<f64> = None;

        loop {
            self.skip_ws();
            if self.consume_char(')') {
                break;
            }

            let key = self.parse_identifier()?;
            self.skip_ws();
            self.expect_char('=')?;
            self.skip_ws();

            match key.as_str() {
                "format" => format = Some(self.parse_identifier_or_quoted_string()?),
                "max_dim" if image => {
                    max_dim = Some(self.parse_u32()?);
                }
                "max_duration" if !image => {
                    let value = self.parse_number()?;
                    if value < 0.0 {
                        return Err(self.err("max_duration must be >= 0"));
                    }
                    max_duration = Some(value);
                }
                _ => return Err(self.err("Unsupported media type constraint key")),
            }

            self.skip_ws();
            if self.consume_char(',') {
                continue;
            }
            if self.consume_char(')') {
                break;
            }
            return Err(self.err("Expected ',' or ')' in media constraints"));
        }

        Ok((format, max_dim, max_duration))
    }

    fn parse_identifier_or_quoted_string(&mut self) -> ValidationResult<String> {
        self.skip_ws();
        if self.consume_char('"') {
            let mut out = String::new();
            loop {
                let ch = self
                    .bump()
                    .ok_or_else(|| self.err("Unterminated string literal"))?;
                if ch == '"' {
                    break;
                }
                if ch == '\\' {
                    let escaped = self.bump().ok_or_else(|| self.err("Unterminated escape"))?;
                    let normalized = match escaped {
                        '"' => '"',
                        '\\' => '\\',
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        other => other,
                    };
                    out.push(normalized);
                } else {
                    out.push(ch);
                }
            }
            Ok(out)
        } else {
            self.parse_identifier()
        }
    }

    fn parse_identifier(&mut self) -> ValidationResult<String> {
        self.skip_ws();
        let rem = self.remaining();
        let mut chars = rem.char_indices();
        let Some((_, first)) = chars.next() else {
            return Err(self.err("Expected identifier"));
        };
        if !(first.is_ascii_alphabetic() || first == '_') {
            return Err(self.err("Expected identifier"));
        }

        let mut end = first.len_utf8();
        for (idx, ch) in chars {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                end = idx + ch.len_utf8();
            } else {
                break;
            }
        }

        let ident = rem[..end].to_string();
        self.offset += end;
        Ok(ident)
    }

    fn parse_usize(&mut self) -> ValidationResult<usize> {
        self.skip_ws();
        let rem = self.remaining();
        let mut end = 0usize;
        for (idx, ch) in rem.char_indices() {
            if ch.is_ascii_digit() {
                end = idx + ch.len_utf8();
            } else {
                break;
            }
        }
        if end == 0 {
            return Err(self.err("Expected positive integer"));
        }
        let parsed = rem[..end]
            .parse::<usize>()
            .map_err(|_| self.err("Invalid integer literal"))?;
        self.offset += end;
        Ok(parsed)
    }

    fn parse_u32(&mut self) -> ValidationResult<u32> {
        let value = self.parse_usize()?;
        u32::try_from(value).map_err(|_| self.err("Integer literal is out of range for u32"))
    }

    fn parse_number(&mut self) -> ValidationResult<f64> {
        self.skip_ws();
        let rem = self.remaining();
        let mut end = 0usize;
        for (idx, ch) in rem.char_indices() {
            if ch.is_ascii_digit() || matches!(ch, '-' | '+' | '.' | 'e' | 'E') {
                end = idx + ch.len_utf8();
            } else {
                break;
            }
        }
        if end == 0 {
            return Err(self.err("Expected number"));
        }
        let parsed = rem[..end]
            .parse::<f64>()
            .map_err(|_| self.err("Invalid number literal"))?;
        self.offset += end;
        Ok(parsed)
    }

    fn starts_keyword(&self, keyword: &str) -> bool {
        let rem = self.remaining();
        if !rem.starts_with(keyword) {
            return false;
        }
        !matches!(
            rem[keyword.len()..].chars().next(),
            Some(ch) if ch.is_ascii_alphanumeric() || ch == '_'
        )
    }

    fn expect_keyword(&mut self, keyword: &str) -> ValidationResult<()> {
        if self.starts_keyword(keyword) {
            self.offset += keyword.len();
            Ok(())
        } else {
            Err(self.err(&format!("Expected keyword '{keyword}'")))
        }
    }

    fn expect_char(&mut self, expected: char) -> ValidationResult<()> {
        if self.consume_char(expected) {
            Ok(())
        } else {
            Err(self.err(&format!("Expected '{expected}'")))
        }
    }

    fn consume_char(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek_char(), Some(c) if c.is_whitespace()) {
            self.bump();
        }
    }

    fn is_eof(&self) -> bool {
        self.offset >= self.input.len()
    }

    fn remaining(&self) -> &str {
        &self.input[self.offset..]
    }

    fn peek_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }

    fn err(&self, message: &str) -> ValidationError {
        ValidationError::ConstraintViolation {
            constraint: "invalid type expression".to_string(),
            value: format!("{message}: {}", self.input),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TypeChecker;
    use crate::errors::ValidationError;
    use fct_ast::{
        BodyNode, FacetBlock, FacetDocument, FacetNode, KeyValueNode, LensCallNode, OrderedMap,
        PipelineNode, ScalarValue, Span, ValueNode,
    };

    fn span() -> Span {
        Span {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        }
    }

    fn lens(name: &str, args: Vec<ValueNode>) -> LensCallNode {
        LensCallNode {
            name: name.to_string(),
            args,
            kwargs: OrderedMap::new(),
            span: span(),
        }
    }

    fn vars_doc(entries: Vec<(&str, ValueNode)>) -> FacetDocument {
        let body = entries
            .into_iter()
            .map(|(key, value)| {
                BodyNode::KeyValue(KeyValueNode {
                    key: key.to_string(),
                    key_kind: Default::default(),
                    value,
                    span: span(),
                })
            })
            .collect::<Vec<_>>();

        FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body,
                span: span(),
            })],
            span: span(),
        }
    }

    #[test]
    fn pipeline_step_type_mismatch_returns_f451() {
        let value = ValueNode::Pipeline(PipelineNode {
            initial: Box::new(ValueNode::String("a,b".to_string())),
            lenses: vec![
                lens("split", vec![ValueNode::String(",".to_string())]),
                lens("trim", vec![]),
            ],
            span: span(),
        });
        let doc = vars_doc(vec![("broken", value)]);

        let mut checker = TypeChecker::new();
        let err = checker
            .validate(&doc)
            .expect_err("incompatible lens chain must fail");

        match err {
            ValidationError::TypeMismatch { location, .. } => {
                assert!(location.contains("trim"));
            }
            other => panic!("expected F451 TypeMismatch, got: {other:?}"),
        }
    }

    #[test]
    fn pipeline_step_type_assignability_accepts_compatible_chain() {
        let value = ValueNode::Pipeline(PipelineNode {
            initial: Box::new(ValueNode::String(" hello ".to_string())),
            lenses: vec![lens("trim", vec![]), lens("uppercase", vec![])],
            span: span(),
        });
        let doc = vars_doc(vec![("ok", value)]);

        let mut checker = TypeChecker::new();
        checker
            .validate(&doc)
            .expect("compatible lens chain should validate");
    }

    #[test]
    fn pipeline_uses_variable_type_for_step_assignability() {
        let doc = vars_doc(vec![
            ("n", ValueNode::Scalar(ScalarValue::Int(7))),
            (
                "broken",
                ValueNode::Pipeline(PipelineNode {
                    initial: Box::new(ValueNode::Variable("n".to_string())),
                    lenses: vec![lens("trim", vec![])],
                    span: span(),
                }),
            ),
        ]);

        let mut checker = TypeChecker::new();
        let err = checker.validate(&doc).expect_err("int |> trim must fail");

        assert!(
            matches!(err, ValidationError::TypeMismatch { .. }),
            "expected F451 TypeMismatch, got: {err:?}"
        );
    }
}
