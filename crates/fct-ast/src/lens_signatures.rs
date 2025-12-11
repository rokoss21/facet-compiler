// ============================================================================
// LENS SIGNATURE SYSTEM
// ============================================================================

use crate::types::{FacetType, ParameterSignature, PrimitiveType};

/// Lens signature containing type information
#[derive(Debug, Clone, PartialEq)]
pub struct LensSignature {
    /// Name of the lens
    pub name: String,
    /// Input type requirement
    pub input_type: FacetType,
    /// Output type guarantee
    pub output_type: FacetType,
    /// Required parameters with their types
    pub parameters: Vec<ParameterSignature>,
    /// Whether the lens is variadic (accepts additional parameters)
    pub variadic: bool,
    /// Optional parameter type for variadic arguments
    pub variadic_type: Option<FacetType>,
}

impl LensSignature {
    /// Create a new lens signature
    pub fn new(
        name: String,
        input_type: FacetType,
        output_type: FacetType,
        parameters: Vec<ParameterSignature>,
    ) -> Self {
        Self {
            name,
            input_type,
            output_type,
            parameters,
            variadic: false,
            variadic_type: None,
        }
    }

    /// Create a variadic lens signature
    pub fn variadic(
        name: String,
        input_type: FacetType,
        output_type: FacetType,
        parameters: Vec<ParameterSignature>,
        variadic_type: Option<FacetType>,
    ) -> Self {
        Self {
            name,
            input_type,
            output_type,
            parameters,
            variadic: true,
            variadic_type,
        }
    }

    /// Get parameter by name
    pub fn get_parameter(&self, name: &str) -> Option<&ParameterSignature> {
        self.parameters.iter().find(|p| p.name == name)
    }

    /// Check if lens accepts the given number of arguments
    pub fn accepts_arg_count(&self, arg_count: usize) -> bool {
        let min_required = self.parameters.iter().filter(|p| p.required).count();

        if self.variadic {
            arg_count >= min_required
        } else {
            arg_count >= min_required && arg_count <= self.parameters.len()
        }
    }

    /// Validate lens call against signature
    pub fn validate_call(&self, args: &[FacetType], kwargs: &[(String, FacetType)]) -> bool {
        // Check argument count
        if !self.accepts_arg_count(args.len()) {
            return false;
        }

        // Check positional arguments
        for (i, arg_type) in args.iter().enumerate() {
            let expected_type = if i < self.parameters.len() {
                &self.parameters[i].param_type
            } else if self.variadic {
                self.variadic_type.as_ref().unwrap_or(&FacetType::Any)
            } else {
                return false;
            };

            if !expected_type.accepts(arg_type) {
                return false;
            }
        }

        // Check keyword arguments
        for (name, arg_type) in kwargs {
            if let Some(param) = self.get_parameter(name) {
                if !param.param_type.accepts(arg_type) {
                    return false;
                }
            } else if !self.variadic {
                return false; // Unknown parameter
            }
        }

        true
    }
}

/// Trait for providing lens signatures
pub trait LensSignatureProvider {
    /// Get lens signature by name
    fn get_signature(&self, lens_name: &str) -> Option<&LensSignature>;

    /// Check if lens exists
    fn has_lens(&self, lens_name: &str) -> bool {
        self.get_signature(lens_name).is_some()
    }

    /// Get all available lens names
    fn lens_names(&self) -> Vec<String>;

    /// Validate a lens call using signatures
    fn validate_lens_call(
        &self,
        lens_name: &str,
        input_type: &FacetType,
        args: &[FacetType],
        kwargs: &[(String, FacetType)],
    ) -> Result<FacetType, String> {
        let signature = self.get_signature(lens_name)
            .ok_or_else(|| format!("Unknown lens: {}", lens_name))?;

        // Check input type compatibility
        if !signature.input_type.accepts(input_type) {
            return Err(format!(
                "Type mismatch for lens '{}': expected {}, got {}",
                lens_name,
                signature.input_type,
                input_type
            ));
        }

        // Validate arguments
        if !signature.validate_call(args, kwargs) {
            return Err(format!(
                "Invalid arguments for lens '{}'",
                lens_name
            ));
        }

        // Return output type
        Ok(signature.output_type.clone())
    }
}

/// Memory-based lens signature registry
#[derive(Debug, Default)]
pub struct LensSignatureRegistry {
    signatures: std::collections::HashMap<String, LensSignature>,
}

impl LensSignatureRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            signatures: std::collections::HashMap::new(),
        }
    }

    /// Register a lens signature
    pub fn register(&mut self, signature: LensSignature) {
        let name = signature.name.clone();
        self.signatures.insert(name, signature);
    }

    /// Register multiple signatures
    pub fn register_all(&mut self, signatures: impl IntoIterator<Item = LensSignature>) {
        for signature in signatures {
            self.register(signature);
        }
    }

    /// Create a registry with standard lens signatures
    pub fn with_standard_lenses() -> Self {
        let mut registry = Self::new();
        registry.register_standard_lenses();
        registry
    }

    /// Register standard FACET lens signatures
    fn register_standard_lenses(&mut self) {
        // Data transformation lenses
        self.register(LensSignature::new(
            "map".to_string(),
            FacetType::List(Box::new(FacetType::Any)),
            FacetType::List(Box::new(FacetType::Any)),
            vec![
                ParameterSignature {
                    name: "function".to_string(),
                    param_type: FacetType::Function,
                    required: true,
                },
            ],
        ));

        self.register(LensSignature::new(
            "filter".to_string(),
            FacetType::List(Box::new(FacetType::Any)),
            FacetType::List(Box::new(FacetType::Any)),
            vec![
                ParameterSignature {
                    name: "predicate".to_string(),
                    param_type: FacetType::Function,
                    required: true,
                },
            ],
        ));

        self.register(LensSignature::new(
            "reduce".to_string(),
            FacetType::List(Box::new(FacetType::Any)),
            FacetType::Any,
            vec![
                ParameterSignature {
                    name: "function".to_string(),
                    param_type: FacetType::Function,
                    required: true,
                },
                ParameterSignature {
                    name: "initial".to_string(),
                    param_type: FacetType::Any,
                    required: false,
                },
            ],
        ));

        // String manipulation lenses (Appendix A.1 from spec)
        self.register(LensSignature::new(
            "trim".to_string(),
            FacetType::Primitive(PrimitiveType::String),
            FacetType::Primitive(PrimitiveType::String),
            vec![],
        ));

        self.register(LensSignature::new(
            "lowercase".to_string(),
            FacetType::Primitive(PrimitiveType::String),
            FacetType::Primitive(PrimitiveType::String),
            vec![],
        ));

        self.register(LensSignature::new(
            "uppercase".to_string(),
            FacetType::Primitive(PrimitiveType::String),
            FacetType::Primitive(PrimitiveType::String),
            vec![],
        ));

        self.register(LensSignature::new(
            "split".to_string(),
            FacetType::Primitive(PrimitiveType::String),
            FacetType::List(Box::new(FacetType::Primitive(PrimitiveType::String))),
            vec![
                ParameterSignature {
                    name: "separator".to_string(),
                    param_type: FacetType::Primitive(PrimitiveType::String),
                    required: false,
                },
            ],
        ));

        self.register(LensSignature::new(
            "replace".to_string(),
            FacetType::Primitive(PrimitiveType::String),
            FacetType::Primitive(PrimitiveType::String),
            vec![
                ParameterSignature {
                    name: "pattern".to_string(),
                    param_type: FacetType::Primitive(PrimitiveType::String),
                    required: true,
                },
                ParameterSignature {
                    name: "replacement".to_string(),
                    param_type: FacetType::Primitive(PrimitiveType::String),
                    required: true,
                },
            ],
        ));

        self.register(LensSignature::new(
            "indent".to_string(),
            FacetType::Primitive(PrimitiveType::String),
            FacetType::Primitive(PrimitiveType::String),
            vec![
                ParameterSignature {
                    name: "level".to_string(),
                    param_type: FacetType::Primitive(PrimitiveType::Number),
                    required: true,
                },
            ],
        ));

        self.register(LensSignature::new(
            "join".to_string(),
            FacetType::List(Box::new(FacetType::Primitive(PrimitiveType::String))),
            FacetType::Primitive(PrimitiveType::String),
            vec![
                ParameterSignature {
                    name: "separator".to_string(),
                    param_type: FacetType::Primitive(PrimitiveType::String),
                    required: false,
                },
            ],
        ));

        // Type conversion lenses
        self.register(LensSignature::new(
            "to_string".to_string(),
            FacetType::Any,
            FacetType::Primitive(PrimitiveType::String),
            vec![],
        ));

        self.register(LensSignature::new(
            "to_number".to_string(),
            FacetType::Primitive(PrimitiveType::String),
            FacetType::Primitive(PrimitiveType::Number),
            vec![],
        ));
    }
}

impl LensSignatureProvider for LensSignatureRegistry {
    fn get_signature(&self, lens_name: &str) -> Option<&LensSignature> {
        self.signatures.get(lens_name)
    }

    fn lens_names(&self) -> Vec<String> {
        self.signatures.keys().cloned().collect()
    }
}