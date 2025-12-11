// ============================================================================
// FACET TYPE SYSTEM
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum FacetType {
    Any,
    Never,
    Primitive(PrimitiveType),
    List(Box<FacetType>),
    Map(Box<FacetType>),
    Struct(Vec<StructField>),
    Union(Vec<FacetType>),
    Function,
    Image { max_dim: Option<u32>, format: Option<String> },
    Audio { max_duration: Option<f64>, format: Option<String> },
    Embedding { size: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveType {
    String,
    Number,
    Boolean,
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub field_type: FacetType,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParameterSignature {
    pub name: String,
    pub param_type: FacetType,
    pub required: bool,
}

impl FacetType {
    /// Check if this type accepts the other type (Liskov substitution)
    pub fn accepts(&self, other: &FacetType) -> bool {
        match (self, other) {
            (FacetType::Any, _) => true,
            (_, FacetType::Any) => true,
            (FacetType::Never, _) => false,
            (_, FacetType::Never) => false,
            (FacetType::Primitive(a), FacetType::Primitive(b)) => a == b,
            (FacetType::List(a), FacetType::List(b)) => a.accepts(b),
            (FacetType::Map(a), FacetType::Map(b)) => a.accepts(b),
            (FacetType::Struct(a), FacetType::Struct(b)) => {
                a.len() == b.len() &&
                a.iter().all(|field_a| {
                    b.iter().any(|field_b| {
                        field_a.name == field_b.name &&
                        (!field_a.required || field_b.required) &&
                        field_a.field_type.accepts(&field_b.field_type)
                    })
                })
            },
            (FacetType::Union(a), _) => a.iter().any(|t| t.accepts(other)),
            (_, FacetType::Union(b)) => b.iter().all(|t| self.accepts(t)),
            (FacetType::Image { .. }, FacetType::Image { .. }) => true,
            (FacetType::Audio { .. }, FacetType::Audio { .. }) => true,
            (FacetType::Embedding { size: a }, FacetType::Embedding { size: b }) => a == b,
            _ => false,
        }
    }
}

impl Default for FacetType {
    fn default() -> Self {
        FacetType::Any
    }
}

impl std::fmt::Display for FacetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FacetType::Any => write!(f, "any"),
            FacetType::Never => write!(f, "never"),
            FacetType::Primitive(p) => write!(f, "{}", p),
            FacetType::List(t) => write!(f, "list[{}]", t),
            FacetType::Map(t) => write!(f, "map[{}]", t),
            FacetType::Struct(fields) => {
                write!(f, "{{")?;
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", field.name, field.field_type)?;
                }
                write!(f, "}}")
            },
            FacetType::Union(types) => {
                write!(f, "union[")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 { write!(f, " | ")?; }
                    write!(f, "{}", t)?;
                }
                write!(f, "]")
            },
            FacetType::Function => write!(f, "function"),
            FacetType::Image { max_dim, format } => {
                write!(f, "image")?;
                if let Some(dim) = max_dim {
                    write!(f, "(max_dim={})", dim)?;
                }
                if let Some(fmt) = format {
                    write!(f, "(format={})", fmt)?;
                }
                Ok(())
            },
            FacetType::Audio { max_duration, format } => {
                write!(f, "audio")?;
                if let Some(duration) = max_duration {
                    write!(f, "(max_duration={})", duration)?;
                }
                if let Some(fmt) = format {
                    write!(f, "(format={})", fmt)?;
                }
                Ok(())
            },
            FacetType::Embedding { size } => write!(f, "embedding({})", size),
        }
    }
}

impl std::fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveType::String => write!(f, "string"),
            PrimitiveType::Number => write!(f, "number"),
            PrimitiveType::Boolean => write!(f, "boolean"),
            PrimitiveType::Null => write!(f, "null"),
        }
    }
}