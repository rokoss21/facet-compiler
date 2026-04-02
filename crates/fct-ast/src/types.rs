// ============================================================================
// FACET TYPE SYSTEM
// ============================================================================

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FacetType {
    Any,
    Never,
    Primitive(PrimitiveType),
    List(Box<FacetType>),
    Map(Box<FacetType>),
    Struct(Vec<StructField>),
    Union(Vec<FacetType>),
    Function,
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
    /// Backward-compatible multimodal wrapper used by validator paths.
    Multimodal(MultimodalType),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveType {
    String,
    Int,
    Float,
    Bool,
    Null,
    Any,
    /// Legacy compatibility alias for `int|float`.
    Number,
    /// Legacy compatibility alias for `bool`.
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub field_type: FacetType,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParameterSignature {
    pub name: String,
    pub param_type: FacetType,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageType {
    pub max_dim: Option<u32>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioType {
    pub max_duration: Option<f64>,
    pub format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingType {
    pub size: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MultimodalType {
    Image(ImageType),
    Audio(AudioType),
    Embedding(EmbeddingType),
}

impl FacetType {
    /// Assignability per FTS rules with compatibility handling for legacy aliases.
    pub fn is_assignable_to(&self, other: &FacetType) -> bool {
        if self == other {
            return true;
        }

        if matches!(other, FacetType::Any | FacetType::Primitive(PrimitiveType::Any)) {
            return true;
        }

        match (self, other) {
            (FacetType::Never, _) | (_, FacetType::Never) => false,
            (FacetType::Primitive(actual), FacetType::Primitive(expected)) => {
                primitive_assignable(actual, expected)
            }
            (FacetType::List(actual), FacetType::List(expected)) => {
                actual.is_assignable_to(expected)
            }
            (FacetType::Map(actual), FacetType::Map(expected)) => actual.is_assignable_to(expected),
            (FacetType::Struct(actual), FacetType::Struct(expected)) => {
                expected.iter().all(|expected_field| {
                    let actual_field =
                        actual.iter().find(|field| field.name == expected_field.name);
                    match (expected_field.required, actual_field) {
                        (true, None) => false,
                        (false, None) => true,
                        (_, Some(field)) => field
                            .field_type
                            .is_assignable_to(&expected_field.field_type),
                    }
                })
            }
            (value, FacetType::Union(expected_members)) => {
                expected_members.iter().any(|member| value.is_assignable_to(member))
            }
            (FacetType::Union(actual_members), target) => {
                actual_members.iter().all(|member| member.is_assignable_to(target))
            }
            (FacetType::Multimodal(actual), FacetType::Multimodal(expected)) => {
                multimodal_assignable(actual, expected)
            }
            (FacetType::Image { max_dim, format }, FacetType::Image { max_dim: e_max, format: e_fmt })
            | (
                FacetType::Image { max_dim, format },
                FacetType::Multimodal(MultimodalType::Image(ImageType {
                    max_dim: e_max,
                    format: e_fmt,
                })),
            ) => image_assignable(format.as_ref(), *max_dim, e_fmt.as_ref(), *e_max),
            (
                FacetType::Multimodal(MultimodalType::Image(ImageType { max_dim, format })),
                FacetType::Image {
                    max_dim: e_max,
                    format: e_fmt,
                },
            ) => image_assignable(format.as_ref(), *max_dim, e_fmt.as_ref(), *e_max),
            (
                FacetType::Audio {
                    max_duration,
                    format,
                },
                FacetType::Audio {
                    max_duration: e_duration,
                    format: e_fmt,
                },
            )
            | (
                FacetType::Audio {
                    max_duration,
                    format,
                },
                FacetType::Multimodal(MultimodalType::Audio(AudioType {
                    max_duration: e_duration,
                    format: e_fmt,
                })),
            ) => audio_assignable(format.as_ref(), *max_duration, e_fmt.as_ref(), *e_duration),
            (
                FacetType::Multimodal(MultimodalType::Audio(AudioType {
                    max_duration,
                    format,
                })),
                FacetType::Audio {
                    max_duration: e_duration,
                    format: e_fmt,
                },
            ) => audio_assignable(format.as_ref(), *max_duration, e_fmt.as_ref(), *e_duration),
            (FacetType::Embedding { size: a }, FacetType::Embedding { size: b }) => a == b,
            (
                FacetType::Embedding { size: a },
                FacetType::Multimodal(MultimodalType::Embedding(EmbeddingType { size: b })),
            ) => a == b,
            (
                FacetType::Multimodal(MultimodalType::Embedding(EmbeddingType { size: a })),
                FacetType::Embedding { size: b },
            ) => a == b,
            _ => false,
        }
    }

    /// Backward-compatible method used by existing lens signature checks.
    pub fn accepts(&self, other: &FacetType) -> bool {
        self.is_assignable_to(other)
    }
}

fn primitive_assignable(actual: &PrimitiveType, expected: &PrimitiveType) -> bool {
    match expected {
        PrimitiveType::Any => true,
        PrimitiveType::String => matches!(actual, PrimitiveType::String),
        PrimitiveType::Int => matches!(actual, PrimitiveType::Int),
        PrimitiveType::Float => matches!(actual, PrimitiveType::Float),
        PrimitiveType::Bool | PrimitiveType::Boolean => {
            matches!(actual, PrimitiveType::Bool | PrimitiveType::Boolean)
        }
        PrimitiveType::Null => matches!(actual, PrimitiveType::Null),
        PrimitiveType::Number => matches!(
            actual,
            PrimitiveType::Int | PrimitiveType::Float | PrimitiveType::Number
        ),
    }
}

fn image_assignable(
    actual_format: Option<&String>,
    actual_dim: Option<u32>,
    expected_format: Option<&String>,
    expected_dim: Option<u32>,
) -> bool {
    let format_ok = match (actual_format, expected_format) {
        (_, None) => true,
        (Some(actual), Some(expected)) => actual == expected,
        (None, Some(_)) => false,
    };
    let dim_ok = match (actual_dim, expected_dim) {
        (_, None) => true,
        (Some(actual), Some(expected)) => actual <= expected,
        (None, Some(_)) => false,
    };
    format_ok && dim_ok
}

fn audio_assignable(
    actual_format: Option<&String>,
    actual_duration: Option<f64>,
    expected_format: Option<&String>,
    expected_duration: Option<f64>,
) -> bool {
    let format_ok = match (actual_format, expected_format) {
        (_, None) => true,
        (Some(actual), Some(expected)) => actual == expected,
        (None, Some(_)) => false,
    };
    let duration_ok = match (actual_duration, expected_duration) {
        (_, None) => true,
        (Some(actual), Some(expected)) => actual <= expected,
        (None, Some(_)) => false,
    };
    format_ok && duration_ok
}

fn multimodal_assignable(actual: &MultimodalType, expected: &MultimodalType) -> bool {
    match (actual, expected) {
        (MultimodalType::Image(a), MultimodalType::Image(e)) => {
            image_assignable(a.format.as_ref(), a.max_dim, e.format.as_ref(), e.max_dim)
        }
        (MultimodalType::Audio(a), MultimodalType::Audio(e)) => audio_assignable(
            a.format.as_ref(),
            a.max_duration,
            e.format.as_ref(),
            e.max_duration,
        ),
        (MultimodalType::Embedding(a), MultimodalType::Embedding(e)) => a.size == e.size,
        _ => false,
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
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", field.name, field.field_type)?;
                }
                write!(f, "}}")
            }
            FacetType::Union(types) => {
                write!(f, "union[")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", t)?;
                }
                write!(f, "]")
            }
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
            }
            FacetType::Audio {
                max_duration,
                format,
            } => {
                write!(f, "audio")?;
                if let Some(duration) = max_duration {
                    write!(f, "(max_duration={})", duration)?;
                }
                if let Some(fmt) = format {
                    write!(f, "(format={})", fmt)?;
                }
                Ok(())
            }
            FacetType::Embedding { size } => write!(f, "embedding({})", size),
            FacetType::Multimodal(MultimodalType::Image(image)) => {
                write!(
                    f,
                    "{}",
                    FacetType::Image {
                        max_dim: image.max_dim,
                        format: image.format.clone(),
                    }
                )
            }
            FacetType::Multimodal(MultimodalType::Audio(audio)) => {
                write!(
                    f,
                    "{}",
                    FacetType::Audio {
                        max_duration: audio.max_duration,
                        format: audio.format.clone(),
                    }
                )
            }
            FacetType::Multimodal(MultimodalType::Embedding(embedding)) => {
                write!(f, "embedding({})", embedding.size)
            }
        }
    }
}

impl std::fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveType::String => write!(f, "string"),
            PrimitiveType::Int => write!(f, "int"),
            PrimitiveType::Float => write!(f, "float"),
            PrimitiveType::Bool | PrimitiveType::Boolean => write!(f, "bool"),
            PrimitiveType::Null => write!(f, "null"),
            PrimitiveType::Any => write!(f, "any"),
            PrimitiveType::Number => write!(f, "number"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioType, EmbeddingType, FacetType, ImageType, MultimodalType, PrimitiveType, StructField,
    };

    #[test]
    fn primitive_assignability_supports_normative_and_legacy_numeric_forms() {
        let int_ty = FacetType::Primitive(PrimitiveType::Int);
        let float_ty = FacetType::Primitive(PrimitiveType::Float);
        let number_ty = FacetType::Primitive(PrimitiveType::Number);

        assert!(int_ty.is_assignable_to(&number_ty));
        assert!(float_ty.is_assignable_to(&number_ty));
        assert!(!number_ty.is_assignable_to(&int_ty));
    }

    #[test]
    fn struct_assignability_requires_expected_required_fields() {
        let actual = FacetType::Struct(vec![
            StructField {
                name: "name".to_string(),
                field_type: FacetType::Primitive(PrimitiveType::String),
                required: true,
            },
            StructField {
                name: "age".to_string(),
                field_type: FacetType::Primitive(PrimitiveType::Int),
                required: true,
            },
        ]);
        let expected = FacetType::Struct(vec![StructField {
            name: "name".to_string(),
            field_type: FacetType::Primitive(PrimitiveType::String),
            required: true,
        }]);

        assert!(actual.is_assignable_to(&expected));
    }

    #[test]
    fn multimodal_assignability_checks_constraints() {
        let actual = FacetType::Multimodal(MultimodalType::Image(ImageType {
            max_dim: Some(512),
            format: Some("jpeg".to_string()),
        }));
        let expected = FacetType::Image {
            max_dim: Some(1024),
            format: Some("jpeg".to_string()),
        };
        let wrong_format = FacetType::Multimodal(MultimodalType::Image(ImageType {
            max_dim: Some(1024),
            format: Some("png".to_string()),
        }));

        assert!(actual.is_assignable_to(&expected));
        assert!(!actual.is_assignable_to(&wrong_format));
    }

    #[test]
    fn embedding_assignability_matches_size() {
        let emb_3 = FacetType::Multimodal(MultimodalType::Embedding(EmbeddingType { size: 3 }));
        let emb_4 = FacetType::Embedding { size: 4 };
        assert!(!emb_3.is_assignable_to(&emb_4));
    }

    #[test]
    fn audio_assignability_checks_duration_bounds() {
        let short_audio = FacetType::Multimodal(MultimodalType::Audio(AudioType {
            max_duration: Some(2.0),
            format: Some("wav".to_string()),
        }));
        let longer_limit = FacetType::Audio {
            max_duration: Some(3.0),
            format: Some("wav".to_string()),
        };
        assert!(short_audio.is_assignable_to(&longer_limit));
    }
}
