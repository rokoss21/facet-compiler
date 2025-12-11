// ============================================================================
// MAP LENSES
// ============================================================================

use crate::{Lens, LensContext, LensError, LensResult, LensSignature, TrustLevel};
use fct_ast::ValueNode;
use std::collections::HashMap;

/// keys() - Extract keys from a map as a list
pub struct KeysLens;

impl Lens for KeysLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::Map(map) => {
                let keys: Vec<ValueNode> =
                    map.keys().map(|k| ValueNode::String(k.clone())).collect();
                Ok(ValueNode::List(keys))
            }
            _ => Err(LensError::TypeMismatch {
                expected: "map".to_string(),
                got: format!("{:?}", input),
            }),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "keys".to_string(),
            input_type: "map".to_string(),
            output_type: "list<string>".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// values() - Extract values from a map as a list
pub struct ValuesLens;

impl Lens for ValuesLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::Map(map) => {
                let values: Vec<ValueNode> = map.values().cloned().collect();
                Ok(ValueNode::List(values))
            }
            _ => Err(LensError::TypeMismatch {
                expected: "map".to_string(),
                got: format!("{:?}", input),
            }),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "values".to_string(),
            input_type: "map".to_string(),
            output_type: "list<any>".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}
