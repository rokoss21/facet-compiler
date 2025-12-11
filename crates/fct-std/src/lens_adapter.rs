// ============================================================================
// LENS REGISTRY ADAPTER
// ============================================================================

//! Adapter for bridging fct-std LensRegistry with the new LensSignatureProvider trait

use crate::LensRegistry;
use fct_ast::{LensSignature, LensSignatureProvider, FacetType, ParameterSignature, PrimitiveType};
use std::collections::HashMap;

/// Adapter that makes fct-std's LensRegistry compatible with LensSignatureProvider
pub struct LensRegistryAdapter {
    registry: LensRegistry,
    signatures: HashMap<String, LensSignature>,
}

impl LensRegistryAdapter {
    /// Create a new adapter from an existing LensRegistry
    pub fn new(registry: LensRegistry) -> Self {
        let mut adapter = Self {
            registry,
            signatures: HashMap::new(),
        };
        adapter.build_signatures();
        adapter
    }

    /// Create an adapter with the standard lens registry
    pub fn with_standard_lenses() -> Self {
        Self::new(LensRegistry::new())
    }

    /// Build signature cache for all lenses in the registry
    fn build_signatures(&mut self) {
        for lens_name in self.registry.list_lenses() {
            let signature = self.create_signature_for_lens(&lens_name);
            self.signatures.insert(lens_name, signature);
        }
    }

    /// Get a reference to the underlying registry
    pub fn inner(&self) -> &LensRegistry {
        &self.registry
    }

    /// Get a mutable reference to the underlying registry
    pub fn inner_mut(&mut self) -> &mut LensRegistry {
        &mut self.registry
    }
}

impl LensSignatureProvider for LensRegistryAdapter {
    fn get_signature(&self, lens_name: &str) -> Option<&LensSignature> {
        self.signatures.get(lens_name)
    }

    fn lens_names(&self) -> Vec<String> {
        self.registry.list_lenses()
    }
}

impl LensRegistryAdapter {
    /// Create a lens signature for a lens based on its name
    fn create_signature_for_lens(&self, lens_name: &str) -> LensSignature {
        // This is a simplified signature creation
        // In a full implementation, we'd parse the actual signature from the lens info
        match lens_name {
            "map" => LensSignature::new(
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
            ),
            "filter" => LensSignature::new(
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
            ),
            "split" => LensSignature::new(
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
            ),
            "join" => LensSignature::new(
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
            ),
            "to_string" => LensSignature::new(
                "to_string".to_string(),
                FacetType::Any,
                FacetType::Primitive(PrimitiveType::String),
                vec![],
            ),
            "to_number" => LensSignature::new(
                "to_number".to_string(),
                FacetType::Primitive(PrimitiveType::String),
                FacetType::Primitive(PrimitiveType::Number),
                vec![],
            ),
            _ => LensSignature::new(
                lens_name.to_string(),
                FacetType::Any,
                FacetType::Any,
                vec![],
            ),
        }
    }
}

/// Extension trait to easily convert LensRegistry to LensSignatureProvider
pub trait LensRegistryExt {
    /// Convert this registry to a LensSignatureProvider
    fn as_signature_provider(self) -> LensRegistryAdapter;
}

impl LensRegistryExt for LensRegistry {
    fn as_signature_provider(self) -> LensRegistryAdapter {
        LensRegistryAdapter::new(self)
    }
}