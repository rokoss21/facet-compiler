#[allow(unused_imports)]
use fct_ast::{ValueNode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Error, Debug)]
pub enum LensError {
    #[error("Type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String },

    #[error("Argument error: {message}")]
    ArgumentError { message: String },

    #[error("Execution error: {message}")]
    ExecutionError { message: String },
}

pub type LensResult<T> = Result<T, LensError>;

// ============================================================================
// LENS TYPE SYSTEM
// ============================================================================

/// Trust level for lenses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Pure lenses - no I/O, deterministic
    Pure = 0,
    /// Bounded external lenses - external calls, deterministic
    Bounded = 1,
    /// Volatile lenses - non-deterministic
    Volatile = 2,
}

/// Lens signature for type checking
#[derive(Debug, Clone)]
pub struct LensSignature {
    pub name: String,
    pub input_type: String, // Simplified for now, could be FacetType
    pub output_type: String,
    pub trust_level: TrustLevel,
    pub deterministic: bool,
}

/// Lens execution context
pub struct LensContext {
    pub variables: HashMap<String, ValueNode>,
}

impl LensContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }
}

impl Default for LensContext {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LENS TRAIT
// ============================================================================

/// Main lens trait
pub trait Lens: Send + Sync {
    /// Execute the lens transformation
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
        kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode>;

    /// Get lens signature for type checking
    fn signature(&self) -> LensSignature;
}

// ============================================================================
// LENS IMPLEMENTATIONS (organized by category)
// ============================================================================

mod lenses;

// Re-export all lens types
pub use lenses::{
    level1::{EmbeddingLens, LlmCallLens, RagSearchLens},
    list::{
        EnsureListLens, FilterLens, FirstLens, JoinLens, LastLens, LengthLens, MapLens, NthLens,
        SliceLens, SortByLens, UniqueLens,
    },
    map::{KeysLens, ValuesLens},
    string::{
        CapitalizeLens, IndentLens, LowercaseLens, ReplaceLens, ReverseLens, SplitLens,
        SubstringLens, TrimLens, UppercaseLens,
    },
    utility::{DefaultLens, HashLens, JsonLens, JsonParseLens, TemplateLens, UrlDecodeLens, UrlEncodeLens},
};

// ============================================================================
// LENS REGISTRY
// ============================================================================

/// Registry holding all available lenses
pub struct LensRegistry {
    lenses: HashMap<String, Box<dyn Lens>>,
}

impl LensRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            lenses: HashMap::new(),
        };

        // Register string lenses
        registry.register(Box::new(TrimLens));
        registry.register(Box::new(LowercaseLens));
        registry.register(Box::new(UppercaseLens));
        registry.register(Box::new(SplitLens));
        registry.register(Box::new(ReplaceLens));
        registry.register(Box::new(IndentLens));
        registry.register(Box::new(CapitalizeLens));
        registry.register(Box::new(ReverseLens));
        registry.register(Box::new(SubstringLens));

        // Register list lenses
        registry.register(Box::new(MapLens));
        registry.register(Box::new(FilterLens));
        registry.register(Box::new(SortByLens));
        registry.register(Box::new(EnsureListLens));
        registry.register(Box::new(FirstLens));
        registry.register(Box::new(LastLens));
        registry.register(Box::new(NthLens));
        registry.register(Box::new(SliceLens));
        registry.register(Box::new(LengthLens));
        registry.register(Box::new(UniqueLens));
        registry.register(Box::new(JoinLens));

        // Register map lenses
        registry.register(Box::new(KeysLens));
        registry.register(Box::new(ValuesLens));

        // Register utility lenses
        registry.register(Box::new(DefaultLens));
        registry.register(Box::new(JsonLens));
        registry.register(Box::new(JsonParseLens));
        registry.register(Box::new(UrlEncodeLens));
        registry.register(Box::new(UrlDecodeLens));
        registry.register(Box::new(HashLens));
        registry.register(Box::new(TemplateLens));

        // Register Level 1 lenses (Bounded External)
        registry.register(Box::new(LlmCallLens));
        registry.register(Box::new(EmbeddingLens));
        registry.register(Box::new(RagSearchLens));

        registry
    }

    pub fn register(&mut self, lens: Box<dyn Lens>) {
        let sig = lens.signature();
        self.lenses.insert(sig.name, lens);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Lens> {
        self.lenses.get(name).map(|b| b.as_ref())
    }

    pub fn get_signature(&self, name: &str) -> Option<LensSignature> {
        self.lenses.get(name).map(|lens| lens.signature())
    }

    pub fn list_lenses(&self) -> Vec<String> {
        self.lenses.keys().cloned().collect()
    }
}

impl Default for LensRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use fct_ast::ScalarValue;

    #[test]
    fn test_trim_lens() {
        let lens = TrimLens;
        let input = ValueNode::String("  hello  ".to_string());
        let result = lens
            .execute(input, vec![], HashMap::new(), &LensContext::new())
            .unwrap();

        assert_eq!(result, ValueNode::String("hello".to_string()));
    }

    #[test]
    fn test_lowercase_lens() {
        let lens = LowercaseLens;
        let input = ValueNode::String("HELLO".to_string());
        let result = lens
            .execute(input, vec![], HashMap::new(), &LensContext::new())
            .unwrap();

        assert_eq!(result, ValueNode::String("hello".to_string()));
    }

    #[test]
    fn test_split_lens() {
        let lens = SplitLens;
        let input = ValueNode::String("a,b,c".to_string());
        let args = vec![ValueNode::String(",".to_string())];
        let result = lens
            .execute(input, args, HashMap::new(), &LensContext::new())
            .unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], ValueNode::String("a".to_string()));
                assert_eq!(items[1], ValueNode::String("b".to_string()));
                assert_eq!(items[2], ValueNode::String("c".to_string()));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_replace_lens() {
        let lens = ReplaceLens;
        let input = ValueNode::String("hello world".to_string());
        let args = vec![
            ValueNode::String("world".to_string()),
            ValueNode::String("Rust".to_string()),
        ];
        let result = lens
            .execute(input, args, HashMap::new(), &LensContext::new())
            .unwrap();

        assert_eq!(result, ValueNode::String("hello Rust".to_string()));
    }

    #[test]
    fn test_default_lens() {
        let lens = DefaultLens;

        // Null input should return default
        let input = ValueNode::Scalar(ScalarValue::Null);
        let args = vec![ValueNode::String("default".to_string())];
        let result = lens
            .execute(input, args, HashMap::new(), &LensContext::new())
            .unwrap();
        assert_eq!(result, ValueNode::String("default".to_string()));

        // Non-null input should return input
        let input = ValueNode::String("value".to_string());
        let args = vec![ValueNode::String("default".to_string())];
        let result = lens
            .execute(input, args, HashMap::new(), &LensContext::new())
            .unwrap();
        assert_eq!(result, ValueNode::String("value".to_string()));
    }

    #[test]
    fn test_lens_registry() {
        let registry = LensRegistry::new();

        assert!(registry.get("trim").is_some());
        assert!(registry.get("lowercase").is_some());
        assert!(registry.get("unknown").is_none());

        let lenses = registry.list_lenses();
        assert!(lenses.contains(&"trim".to_string()));
    }

    #[test]
    fn test_keys_lens() {
        let lens = KeysLens;
        let ctx = LensContext {
            variables: HashMap::new(),
        };

        let mut map = HashMap::new();
        map.insert("name".to_string(), ValueNode::String("Alice".to_string()));
        map.insert("age".to_string(), ValueNode::Scalar(ScalarValue::Int(30)));

        let result = lens
            .execute(ValueNode::Map(map), vec![], HashMap::new(), &ctx)
            .unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 2);
                assert!(items.contains(&ValueNode::String("name".to_string())));
                assert!(items.contains(&ValueNode::String("age".to_string())));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_values_lens() {
        let lens = ValuesLens;
        let ctx = LensContext {
            variables: HashMap::new(),
        };

        let mut map = HashMap::new();
        map.insert("name".to_string(), ValueNode::String("Bob".to_string()));
        map.insert("age".to_string(), ValueNode::Scalar(ScalarValue::Int(25)));

        let result = lens
            .execute(ValueNode::Map(map), vec![], HashMap::new(), &ctx)
            .unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 2);
                assert!(items.contains(&ValueNode::String("Bob".to_string())));
                assert!(items.contains(&ValueNode::Scalar(ScalarValue::Int(25))));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_indent_lens() {
        let lens = IndentLens;
        let ctx = LensContext {
            variables: HashMap::new(),
        };

        let input = ValueNode::String("line1\nline2\nline3".to_string());
        let result = lens.execute(input, vec![], HashMap::new(), &ctx).unwrap();

        assert_eq!(
            result,
            ValueNode::String("  line1\n  line2\n  line3".to_string())
        );

        // Test with custom indent
        let input2 = ValueNode::String("a\nb".to_string());
        let result2 = lens
            .execute(
                input2,
                vec![ValueNode::Scalar(ScalarValue::Int(4))],
                HashMap::new(),
                &ctx,
            )
            .unwrap();

        assert_eq!(result2, ValueNode::String("    a\n    b".to_string()));
    }

    #[test]
    fn test_json_lens() {
        let lens = JsonLens;
        let ctx = LensContext {
            variables: HashMap::new(),
        };

        let mut map = HashMap::new();
        map.insert("key".to_string(), ValueNode::String("value".to_string()));
        map.insert("num".to_string(), ValueNode::Scalar(ScalarValue::Int(42)));

        let result = lens
            .execute(ValueNode::Map(map), vec![], HashMap::new(), &ctx)
            .unwrap();

        match result {
            ValueNode::String(s) => {
                assert!(s.contains("\"key\""));
                assert!(s.contains("\"value\""));
                assert!(s.contains("\"num\""));
            }
            _ => panic!("Expected string"),
        }
    }

    #[test]
    fn test_map_lens() {
        let lens = MapLens;
        let ctx = LensContext {
            variables: HashMap::new(),
        };

        // Test with "to_string" operation
        let input_list = vec![
            ValueNode::Scalar(ScalarValue::Int(1)),
            ValueNode::Scalar(ScalarValue::Int(2)),
        ];
        let input = ValueNode::List(input_list);

        let result = lens
            .execute(
                input,
                vec![ValueNode::String("to_string".to_string())],
                HashMap::new(),
                &ctx,
            )
            .unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 2);
                // Check that items are now strings
                match &items[0] {
                    ValueNode::String(s) => assert!(s.contains("Int(1)")),
                    _ => panic!("Expected string"),
                }
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_filter_lens() {
        let lens = FilterLens;
        let ctx = LensContext {
            variables: HashMap::new(),
        };

        // Test filtering non_null
        let input_list = vec![
            ValueNode::String("test".to_string()),
            ValueNode::Scalar(ScalarValue::Null),
            ValueNode::Scalar(ScalarValue::Int(42)),
        ];
        let input = ValueNode::List(input_list);

        let result = lens
            .execute(
                input,
                vec![ValueNode::String("non_null".to_string())],
                HashMap::new(),
                &ctx,
            )
            .unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 2); // Should filter out null
                assert!(!items
                    .iter()
                    .any(|i| matches!(i, ValueNode::Scalar(ScalarValue::Null))));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_sort_by_lens() {
        let lens = SortByLens;
        let ctx = LensContext {
            variables: HashMap::new(),
        };

        let input_list = vec![
            ValueNode::String("zebra".to_string()),
            ValueNode::String("apple".to_string()),
            ValueNode::String("banana".to_string()),
        ];
        let input = ValueNode::List(input_list);

        // Test ascending sort
        let result = lens
            .execute(input.clone(), vec![], HashMap::new(), &ctx)
            .unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 3);
                match &items[0] {
                    ValueNode::String(s) => assert_eq!(s, "apple"),
                    _ => panic!("Expected string"),
                }
            }
            _ => panic!("Expected list"),
        }

        // Test descending sort
        let result_desc = lens
            .execute(
                input,
                vec![
                    ValueNode::String("key".to_string()),
                    ValueNode::String("desc".to_string()),
                ],
                HashMap::new(),
                &ctx,
            )
            .unwrap();

        match result_desc {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 3);
                match &items[0] {
                    ValueNode::String(s) => assert_eq!(s, "zebra"),
                    _ => panic!("Expected string"),
                }
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_ensure_list_lens() {
        let lens = EnsureListLens;
        let ctx = LensContext {
            variables: HashMap::new(),
        };

        // Test with single value
        let input = ValueNode::String("test".to_string());
        let result = lens.execute(input, vec![], HashMap::new(), &ctx).unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0], ValueNode::String("test".to_string()));
            }
            _ => panic!("Expected list"),
        }

        // Test with existing list (should pass through)
        let input_list = vec![
            ValueNode::String("a".to_string()),
            ValueNode::String("b".to_string()),
        ];
        let input = ValueNode::List(input_list);
        let result = lens.execute(input, vec![], HashMap::new(), &ctx).unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 2);
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_all_lenses_in_registry() {
        let registry = LensRegistry::new();
        let lenses = registry.list_lenses();

        // Check that all lenses are registered
        let expected_lenses = vec![
            // String lenses
            "trim",
            "lowercase",
            "uppercase",
            "split",
            "replace",
            "indent",
            "capitalize",
            "reverse",
            "substring",
            // List lenses
            "map",
            "filter",
            "sort_by",
            "ensure_list",
            "first",
            "last",
            "nth",
            "slice",
            "length",
            "unique",
            "join",
            // Map lenses
            "keys",
            "values",
            // Utility lenses
            "default",
            "json",
            "json_parse",
            "url_encode",
            "url_decode",
            "hash",
            "template",
            // Level 1 lenses (Bounded)
            "llm_call",
            "embedding",
            "rag_search",
        ];

        for lens_name in expected_lenses {
            assert!(
                lenses.contains(&lens_name.to_string()),
                "Missing lens: {}",
                lens_name
            );
        }

        assert_eq!(lenses.len(), 32); // 9 string + 11 list + 2 map + 7 utility + 3 level1
    }

    #[test]
    fn test_capitalize_lens() {
        let lens = CapitalizeLens;
        let input = ValueNode::String("hello world".to_string());
        let result = lens
            .execute(input, vec![], HashMap::new(), &LensContext::new())
            .unwrap();

        assert_eq!(result, ValueNode::String("Hello world".to_string()));

        // Test empty string
        let empty_input = ValueNode::String("".to_string());
        let empty_result = lens
            .execute(empty_input, vec![], HashMap::new(), &LensContext::new())
            .unwrap();

        assert_eq!(empty_result, ValueNode::String("".to_string()));
    }

    #[test]
    fn test_reverse_lens() {
        let lens = ReverseLens;
        let input = ValueNode::String("hello".to_string());
        let result = lens
            .execute(input, vec![], HashMap::new(), &LensContext::new())
            .unwrap();

        assert_eq!(result, ValueNode::String("olleh".to_string()));

        // Test with UTF-8
        let utf8_input = ValueNode::String("привет".to_string());
        let utf8_result = lens
            .execute(utf8_input, vec![], HashMap::new(), &LensContext::new())
            .unwrap();

        assert_eq!(utf8_result, ValueNode::String("тевирп".to_string()));
    }

    #[test]
    fn test_substring_lens() {
        let lens = SubstringLens;

        // Test with start only
        let input = ValueNode::String("hello world".to_string());
        let args = vec![ValueNode::Scalar(ScalarValue::Int(6))];
        let result = lens
            .execute(input.clone(), args, HashMap::new(), &LensContext::new())
            .unwrap();

        assert_eq!(result, ValueNode::String("world".to_string()));

        // Test with start and end
        let args2 = vec![
            ValueNode::Scalar(ScalarValue::Int(0)),
            ValueNode::Scalar(ScalarValue::Int(5)),
        ];
        let result2 = lens
            .execute(input.clone(), args2, HashMap::new(), &LensContext::new())
            .unwrap();

        assert_eq!(result2, ValueNode::String("hello".to_string()));

        // Test with UTF-8
        let utf8_input = ValueNode::String("привет мир".to_string());
        let utf8_args = vec![
            ValueNode::Scalar(ScalarValue::Int(0)),
            ValueNode::Scalar(ScalarValue::Int(6)),
        ];
        let utf8_result = lens
            .execute(utf8_input, utf8_args, HashMap::new(), &LensContext::new())
            .unwrap();

        assert_eq!(utf8_result, ValueNode::String("привет".to_string()));
    }

    #[test]
    fn test_first_lens() {
        let lens = FirstLens;
        let ctx = LensContext::new();

        let input = ValueNode::List(vec![
            ValueNode::String("first".to_string()),
            ValueNode::String("second".to_string()),
            ValueNode::String("third".to_string()),
        ]);

        let result = lens.execute(input, vec![], HashMap::new(), &ctx).unwrap();
        assert_eq!(result, ValueNode::String("first".to_string()));

        // Test error on empty list
        let empty_input = ValueNode::List(vec![]);
        let result = lens.execute(empty_input, vec![], HashMap::new(), &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_last_lens() {
        let lens = LastLens;
        let ctx = LensContext::new();

        let input = ValueNode::List(vec![
            ValueNode::String("first".to_string()),
            ValueNode::String("second".to_string()),
            ValueNode::String("third".to_string()),
        ]);

        let result = lens.execute(input, vec![], HashMap::new(), &ctx).unwrap();
        assert_eq!(result, ValueNode::String("third".to_string()));

        // Test error on empty list
        let empty_input = ValueNode::List(vec![]);
        let result = lens.execute(empty_input, vec![], HashMap::new(), &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_nth_lens() {
        let lens = NthLens;
        let ctx = LensContext::new();

        let input = ValueNode::List(vec![
            ValueNode::String("zero".to_string()),
            ValueNode::String("one".to_string()),
            ValueNode::String("two".to_string()),
        ]);

        // Test getting element at index 1
        let result = lens
            .execute(
                input.clone(),
                vec![ValueNode::Scalar(ScalarValue::Int(1))],
                HashMap::new(),
                &ctx,
            )
            .unwrap();
        assert_eq!(result, ValueNode::String("one".to_string()));

        // Test out of bounds error
        let result = lens.execute(
            input,
            vec![ValueNode::Scalar(ScalarValue::Int(10))],
            HashMap::new(),
            &ctx,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_slice_lens() {
        let lens = SliceLens;
        let ctx = LensContext::new();

        let input = ValueNode::List(vec![
            ValueNode::Scalar(ScalarValue::Int(0)),
            ValueNode::Scalar(ScalarValue::Int(1)),
            ValueNode::Scalar(ScalarValue::Int(2)),
            ValueNode::Scalar(ScalarValue::Int(3)),
            ValueNode::Scalar(ScalarValue::Int(4)),
        ]);

        // Test slice with start and end
        let result = lens
            .execute(
                input.clone(),
                vec![
                    ValueNode::Scalar(ScalarValue::Int(1)),
                    ValueNode::Scalar(ScalarValue::Int(4)),
                ],
                HashMap::new(),
                &ctx,
            )
            .unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], ValueNode::Scalar(ScalarValue::Int(1)));
                assert_eq!(items[1], ValueNode::Scalar(ScalarValue::Int(2)));
                assert_eq!(items[2], ValueNode::Scalar(ScalarValue::Int(3)));
            }
            _ => panic!("Expected list"),
        }

        // Test slice with only start (should go to end)
        let result2 = lens
            .execute(
                input,
                vec![ValueNode::Scalar(ScalarValue::Int(3))],
                HashMap::new(),
                &ctx,
            )
            .unwrap();

        match result2 {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], ValueNode::Scalar(ScalarValue::Int(3)));
                assert_eq!(items[1], ValueNode::Scalar(ScalarValue::Int(4)));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_length_lens() {
        let lens = LengthLens;
        let ctx = LensContext::new();

        let input = ValueNode::List(vec![
            ValueNode::String("a".to_string()),
            ValueNode::String("b".to_string()),
            ValueNode::String("c".to_string()),
        ]);

        let result = lens.execute(input, vec![], HashMap::new(), &ctx).unwrap();
        assert_eq!(result, ValueNode::Scalar(ScalarValue::Int(3)));

        // Test empty list
        let empty_input = ValueNode::List(vec![]);
        let empty_result = lens
            .execute(empty_input, vec![], HashMap::new(), &ctx)
            .unwrap();
        assert_eq!(empty_result, ValueNode::Scalar(ScalarValue::Int(0)));
    }

    #[test]
    fn test_unique_lens() {
        let lens = UniqueLens;
        let ctx = LensContext::new();

        let input = ValueNode::List(vec![
            ValueNode::String("apple".to_string()),
            ValueNode::String("banana".to_string()),
            ValueNode::String("apple".to_string()),
            ValueNode::String("cherry".to_string()),
            ValueNode::String("banana".to_string()),
        ]);

        let result = lens.execute(input, vec![], HashMap::new(), &ctx).unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], ValueNode::String("apple".to_string()));
                assert_eq!(items[1], ValueNode::String("banana".to_string()));
                assert_eq!(items[2], ValueNode::String("cherry".to_string()));
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_join_lens() {
        let lens = JoinLens;
        let ctx = LensContext::new();

        // Test with string separator
        let input = ValueNode::List(vec![
            ValueNode::String("hello".to_string()),
            ValueNode::String("world".to_string()),
            ValueNode::String("test".to_string()),
        ]);

        let result = lens
            .execute(
                input.clone(),
                vec![ValueNode::String(", ".to_string())],
                HashMap::new(),
                &ctx,
            )
            .unwrap();
        assert_eq!(result, ValueNode::String("hello, world, test".to_string()));

        // Test with no separator (default empty string)
        let result2 = lens
            .execute(input, vec![], HashMap::new(), &ctx)
            .unwrap();
        assert_eq!(result2, ValueNode::String("helloworldtest".to_string()));

        // Test with mixed types
        let mixed_input = ValueNode::List(vec![
            ValueNode::String("text".to_string()),
            ValueNode::Scalar(ScalarValue::Int(42)),
            ValueNode::Scalar(ScalarValue::Bool(true)),
        ]);

        let result3 = lens
            .execute(
                mixed_input,
                vec![ValueNode::String("-".to_string())],
                HashMap::new(),
                &ctx,
            )
            .unwrap();
        assert_eq!(result3, ValueNode::String("text-42-true".to_string()));
    }

    #[test]
    fn test_json_parse_lens() {
        let lens = JsonParseLens;
        let ctx = LensContext::new();

        // Test parsing JSON object
        let json_input = ValueNode::String(r#"{"name":"Alice","age":30}"#.to_string());
        let result = lens
            .execute(json_input, vec![], HashMap::new(), &ctx)
            .unwrap();

        match result {
            ValueNode::Map(map) => {
                assert_eq!(map.get("name"), Some(&ValueNode::String("Alice".to_string())));
                assert_eq!(map.get("age"), Some(&ValueNode::Scalar(ScalarValue::Int(30))));
            }
            _ => panic!("Expected map"),
        }

        // Test parsing JSON array
        let array_input = ValueNode::String(r#"[1,2,3]"#.to_string());
        let array_result = lens
            .execute(array_input, vec![], HashMap::new(), &ctx)
            .unwrap();

        match array_result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected list"),
        }
    }

    #[test]
    fn test_url_encode_lens() {
        let lens = UrlEncodeLens;
        let ctx = LensContext::new();

        // Test encoding special characters
        let input = ValueNode::String("hello world!".to_string());
        let result = lens
            .execute(input, vec![], HashMap::new(), &ctx)
            .unwrap();

        assert_eq!(result, ValueNode::String("hello%20world%21".to_string()));

        // Test encoding with more complex characters
        let input2 = ValueNode::String("a+b=c&d".to_string());
        let result2 = lens
            .execute(input2, vec![], HashMap::new(), &ctx)
            .unwrap();

        match result2 {
            ValueNode::String(s) => {
                assert!(s.contains("%"));
            }
            _ => panic!("Expected string"),
        }
    }

    #[test]
    fn test_url_decode_lens() {
        let lens = UrlDecodeLens;
        let ctx = LensContext::new();

        // Test decoding
        let input = ValueNode::String("hello%20world%21".to_string());
        let result = lens
            .execute(input, vec![], HashMap::new(), &ctx)
            .unwrap();

        assert_eq!(result, ValueNode::String("hello world!".to_string()));

        // Test decoding plus sign
        let input2 = ValueNode::String("hello+world".to_string());
        let result2 = lens
            .execute(input2, vec![], HashMap::new(), &ctx)
            .unwrap();

        assert_eq!(result2, ValueNode::String("hello+world".to_string()));
    }

    #[test]
    fn test_hash_lens() {
        let lens = HashLens;
        let ctx = LensContext::new();

        // Test SHA256 (default)
        let input = ValueNode::String("hello".to_string());
        let result = lens
            .execute(input.clone(), vec![], HashMap::new(), &ctx)
            .unwrap();

        match result {
            ValueNode::String(hash) => {
                assert_eq!(hash.len(), 64); // SHA256 produces 64 hex characters
                assert_eq!(
                    hash,
                    "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
                );
            }
            _ => panic!("Expected string"),
        }

        // Test MD5
        let result_md5 = lens
            .execute(
                input.clone(),
                vec![ValueNode::String("md5".to_string())],
                HashMap::new(),
                &ctx,
            )
            .unwrap();

        match result_md5 {
            ValueNode::String(hash) => {
                assert_eq!(hash.len(), 32); // MD5 produces 32 hex characters
                assert_eq!(hash, "5d41402abc4b2a76b9719d911017c592");
            }
            _ => panic!("Expected string"),
        }

        // Test SHA512
        let result_sha512 = lens
            .execute(
                input,
                vec![ValueNode::String("sha512".to_string())],
                HashMap::new(),
                &ctx,
            )
            .unwrap();

        match result_sha512 {
            ValueNode::String(hash) => {
                assert_eq!(hash.len(), 128); // SHA512 produces 128 hex characters
            }
            _ => panic!("Expected string"),
        }
    }

    #[test]
    fn test_template_lens() {
        let lens = TemplateLens;
        let ctx = LensContext::new();

        // Test basic template substitution
        let input = ValueNode::String("Hello, {{name}}!".to_string());
        let mut kwargs = HashMap::new();
        kwargs.insert("name".to_string(), ValueNode::String("Alice".to_string()));

        let result = lens.execute(input, vec![], kwargs, &ctx).unwrap();

        assert_eq!(result, ValueNode::String("Hello, Alice!".to_string()));

        // Test multiple substitutions
        let input2 = ValueNode::String("{{greeting}}, {{name}}! You are {{age}} years old.".to_string());
        let mut kwargs2 = HashMap::new();
        kwargs2.insert("greeting".to_string(), ValueNode::String("Hi".to_string()));
        kwargs2.insert("name".to_string(), ValueNode::String("Bob".to_string()));
        kwargs2.insert("age".to_string(), ValueNode::Scalar(ScalarValue::Int(25)));

        let result2 = lens.execute(input2, vec![], kwargs2, &ctx).unwrap();

        assert_eq!(
            result2,
            ValueNode::String("Hi, Bob! You are 25 years old.".to_string())
        );

        // Test with no substitutions needed
        let input3 = ValueNode::String("No variables here".to_string());
        let result3 = lens
            .execute(input3.clone(), vec![], HashMap::new(), &ctx)
            .unwrap();

        assert_eq!(result3, input3);
    }

    #[test]
    fn test_llm_call_lens() {
        let lens = LlmCallLens;
        let ctx = LensContext::new();

        // Test basic LLM call
        let input = ValueNode::String("What is the meaning of life?".to_string());
        let result = lens
            .execute(input.clone(), vec![], HashMap::new(), &ctx)
            .unwrap();

        match result {
            ValueNode::String(s) => {
                assert!(s.contains("[STUB]"));
                assert!(s.contains("gpt-3.5-turbo")); // Default model
            }
            _ => panic!("Expected string"),
        }

        // Test with custom model
        let result2 = lens
            .execute(
                input.clone(),
                vec![ValueNode::String("gpt-4".to_string())],
                HashMap::new(),
                &ctx,
            )
            .unwrap();

        match result2 {
            ValueNode::String(s) => {
                assert!(s.contains("gpt-4"));
            }
            _ => panic!("Expected string"),
        }

        // Test with kwargs
        let mut kwargs = HashMap::new();
        kwargs.insert(
            "temperature".to_string(),
            ValueNode::Scalar(ScalarValue::Float(0.9)),
        );
        kwargs.insert(
            "max_tokens".to_string(),
            ValueNode::Scalar(ScalarValue::Int(500)),
        );

        let result3 = lens.execute(input, vec![], kwargs, &ctx).unwrap();

        match result3 {
            ValueNode::String(s) => {
                assert!(s.contains("temp=0.9"));
                assert!(s.contains("max_tokens=500"));
            }
            _ => panic!("Expected string"),
        }

        // Verify trust level is Bounded
        assert_eq!(lens.signature().trust_level, TrustLevel::Bounded);
        assert!(!lens.signature().deterministic);
    }

    #[test]
    fn test_embedding_lens() {
        let lens = EmbeddingLens;
        let ctx = LensContext::new();

        // Test basic embedding generation
        let input = ValueNode::String("Hello world".to_string());
        let result = lens.execute(input, vec![], HashMap::new(), &ctx).unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 10); // Stub returns 10 floats
                // Verify all items are floats
                for item in items {
                    match item {
                        ValueNode::Scalar(ScalarValue::Float(_)) => {}
                        _ => panic!("Expected float"),
                    }
                }
            }
            _ => panic!("Expected list"),
        }

        // Test with custom model
        let input2 = ValueNode::String("Test text".to_string());
        let result2 = lens
            .execute(
                input2,
                vec![ValueNode::String("text-embedding-3-large".to_string())],
                HashMap::new(),
                &ctx,
            )
            .unwrap();

        match result2 {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 10);
            }
            _ => panic!("Expected list"),
        }

        // Verify trust level is Bounded
        assert_eq!(lens.signature().trust_level, TrustLevel::Bounded);
        assert!(!lens.signature().deterministic);
    }

    #[test]
    fn test_rag_search_lens() {
        let lens = RagSearchLens;
        let ctx = LensContext::new();

        // Test basic RAG search
        let input = ValueNode::String("machine learning".to_string());
        let result = lens
            .execute(
                input.clone(),
                vec![ValueNode::String("my-index".to_string())],
                HashMap::new(),
                &ctx,
            )
            .unwrap();

        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 5); // Default top_k=5
                // Verify each result is a map with content and score
                for item in items {
                    match item {
                        ValueNode::Map(m) => {
                            assert!(m.contains_key("content"));
                            assert!(m.contains_key("score"));
                        }
                        _ => panic!("Expected map"),
                    }
                }
            }
            _ => panic!("Expected list"),
        }

        // Test with custom top_k
        let mut kwargs = HashMap::new();
        kwargs.insert("top_k".to_string(), ValueNode::Scalar(ScalarValue::Int(3)));

        let result2 = lens
            .execute(
                input,
                vec![ValueNode::String("my-index".to_string())],
                kwargs,
                &ctx,
            )
            .unwrap();

        match result2 {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 3);
            }
            _ => panic!("Expected list"),
        }

        // Verify trust level is Bounded
        assert_eq!(lens.signature().trust_level, TrustLevel::Bounded);
        assert!(!lens.signature().deterministic);
    }
}

// ============================================================================
// LENS ADAPTER MODULE
// ============================================================================

pub mod lens_adapter;
pub use lens_adapter::{LensRegistryAdapter, LensRegistryExt};
