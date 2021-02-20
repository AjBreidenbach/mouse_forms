use std::fmt;
#[derive(Debug)]
pub enum SyntacticError {
    MismatchedTags {
        open_tag: Option<String>,
        closing_tag: String,
    },
    InvalidAttribute {
        attribute_name: String,
        context: String,
    },
    InvalidFieldType {
        invalid_type: String,
    },
    InvalidGroupType {
        invalid_type: String,
    },
    OrphanElement {
        context: String,
    },
    UnnamedElement {
        context: String,
    },
    ImproperNesting {
        context: String,
    },
}

impl std::error::Error for SyntacticError {}

impl fmt::Display for SyntacticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            SyntacticError::MismatchedTags {
                open_tag,
                closing_tag,
            } => write!(
                f,
                "expected matching opening tag for {}, but got {:?}",
                closing_tag, open_tag
            ),
            SyntacticError::InvalidAttribute {
                attribute_name,
                context,
            } => write!(
                f,
                "encountered invalid attribute name {} in {}",
                attribute_name, context
            ),
            SyntacticError::InvalidFieldType { invalid_type } => {
                write!(f, "invalid field type {}", invalid_type)
            }
            SyntacticError::InvalidGroupType { invalid_type } => {
                write!(f, "invalid group type {}", invalid_type)
            }
            e => write!(f, "{:?}", e),
        }
    }
}
