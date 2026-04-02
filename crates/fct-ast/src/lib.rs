use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

pub type OrderedMap<K, V> = IndexMap<K, V>;
pub const FACET_VERSION: &str = "2.1.3";
pub const POLICY_VERSION: &str = "1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FacetDocument {
    pub blocks: Vec<FacetNode>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum FacetNode {
    Meta(FacetBlock),
    System(FacetBlock),
    User(FacetBlock),
    Assistant(FacetBlock),
    Vars(FacetBlock),
    VarTypes(FacetBlock),
    Context(FacetBlock),
    Policy(FacetBlock),
    Import(ImportNode),
    Interface(InterfaceNode),
    Test(TestBlock),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FacetBlock {
    pub name: String,
    pub attributes: OrderedMap<String, ValueNode>,
    pub body: Vec<BodyNode>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportNode {
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceNode {
    pub name: String,
    pub functions: Vec<FunctionSignature>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: TypeNode,
    pub effect: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub type_node: TypeNode,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestBlock {
    pub name: String,
    pub vars: OrderedMap<String, ValueNode>,
    #[serde(default)]
    pub input: OrderedMap<String, ValueNode>,
    pub mocks: Vec<MockDefinition>,
    pub assertions: Vec<Assertion>,
    pub body: Vec<BodyNode>, // Keep for backward compatibility
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MockDefinition {
    pub target: String, // e.g., "WeatherAPI.get_current"
    pub return_value: ValueNode,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assertion {
    pub kind: AssertionKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum AssertionKind {
    Contains { target: String, text: String },
    NotContains { target: String, text: String },
    Equals { target: String, expected: ValueNode },
    NotEquals { target: String, expected: ValueNode },
    LessThan { field: String, value: f64 },
    GreaterThan { field: String, value: f64 },
    Sentiment { target: String, expected: String },
    Matches { target: String, pattern: String },
    NotMatches { target: String, pattern: String },
    True { target: String },
    False { target: String },
    Null { target: String },
    NotNull { target: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BodyNode {
    KeyValue(KeyValueNode),
    ListItem(ListItemNode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MapKeyKind {
    #[default]
    Identifier,
    String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KeyValueNode {
    pub key: String,
    #[serde(default)]
    pub key_kind: MapKeyKind,
    pub value: ValueNode,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItemNode {
    pub value: ValueNode,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum ValueNode {
    Scalar(ScalarValue),
    String(String),
    Variable(String), // $foo.bar
    Pipeline(PipelineNode),
    List(Vec<ValueNode>),
    Map(OrderedMap<String, ValueNode>),
    Directive(DirectiveNode),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ScalarValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineNode {
    pub initial: Box<ValueNode>,
    pub lenses: Vec<LensCallNode>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LensCallNode {
    pub name: String,
    pub args: Vec<ValueNode>,
    pub kwargs: OrderedMap<String, ValueNode>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectiveNode {
    pub name: String,
    pub args: OrderedMap<String, ValueNode>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeNode {
    Primitive(String),
    Struct(OrderedMap<String, TypeNode>),
    List(Box<TypeNode>),
    Map(Box<TypeNode>),
    Union(Vec<TypeNode>),
    Image {
        max_dim: Option<u32>,
        format: Option<String>,
    },
    Audio {
        max_duration: Option<f64>,
        format: Option<String>,
    },
    Embedding {
        size: usize,
    },
}

// Export type system and lens signature system
pub mod lens_signatures;
pub mod types;

pub use lens_signatures::{LensSignature, LensSignatureProvider, LensSignatureRegistry};
pub use types::{FacetType, ParameterSignature, PrimitiveType, StructField};
