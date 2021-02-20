mod error;
pub use error::SyntacticError;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use xml::attribute::OwnedAttribute;

#[derive(Serialize, Deserialize, Debug)]
pub struct Form {
    pub title: Option<String>,
    pub unlisted: bool,
    pub description: Option<String>,
    pub meta_description: Option<String>,
    pub dir_description: Option<String>,
    pub embedded_scripts: Vec<String>,
    pub category: Option<String>,
    pub instructions: Option<String>,
    pub link: Option<String>,
    pub index: u16,
    pub stylesheet: Option<String>,
    pub sections: Vec<Section>,
    pub language: Option<String>,
    pub keywords: Option<String>,
}

impl Form {
    pub fn new() -> Self {
        Form {
            title: None,
            unlisted: false,
            description: None,
            meta_description: None,
            dir_description: None,
            category: None,
            link: None,
            instructions: None,
            index: std::u16::MAX,
            embedded_scripts: Vec::with_capacity(0),
            stylesheet: None,
            sections: vec![],
            language: None,
            keywords: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Section {
    pub name: String,
    pub title: Option<String>,
    pub instructions: Option<String>,
    pub elements: Vec<FormElement>,
    attributes: ElementAttributes,
}

impl TryFrom<&Vec<OwnedAttribute>> for Section {
    type Error = SyntacticError;
    fn try_from(attributes: &Vec<OwnedAttribute>) -> Result<Self, Self::Error> {
        let mut name = None;
        let mut self_attributes = ElementAttributes::new();
        let context = String::from("section; attribute is unrecognized");

        for attribute in attributes {
            let attribute_name = &attribute.name.local_name;
            let value = &attribute.value;

            match attribute_name.as_str() {
                "name" => name = Some(value.clone()),
                _ => self_attributes.try_apply(&attribute_name, &value, &context)?,
            }
        }
        let name = name.ok_or_else(|| SyntacticError::UnnamedElement {
            context: String::from("section must have a name"),
        })?;

        Ok(Self {
            attributes: self_attributes,
            name,
            instructions: None,
            title: None,
            elements: Vec::new(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ElementAttributes {
    requires: Option<String>,
    optional: bool,
    optional_if: Option<String>,
    class: Option<String>,
}

impl ElementAttributes {
    fn new() -> Self {
        Self {
            requires: None,
            optional: false,
            optional_if: None,
            class: None,
        }
    }

    fn try_apply(
        &mut self,
        attribute_name: &String,
        value: &String,
        context: &String,
    ) -> Result<(), SyntacticError> {
        match attribute_name.as_str() {
            "requires" => self.requires = Some(value.clone()),
            "optional" => self.optional = true,
            "optional-if" => self.optional_if = Some(value.clone()),
            "class" => self.class = Some(value.clone()),
            _ => {
                return Err(SyntacticError::InvalidAttribute {
                    attribute_name: attribute_name.clone(),
                    context: context.clone(),
                })
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum FormElement {
    Group(Group),
    Field(Field),
}

#[derive(Serialize, Deserialize, Debug)]
enum GroupType {
    Row,
    Subsection,
}

impl TryFrom<&String> for GroupType {
    type Error = SyntacticError;
    fn try_from(s: &String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "row" => Ok(GroupType::Row),
            "subsection" => Ok(GroupType::Subsection),
            "" => Ok(GroupType::Row),
            _ => Err(SyntacticError::InvalidGroupType {
                invalid_type: s.clone(),
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Group {
    pub name: String,
    pub title: Option<String>,
    pub instructions: Option<String>,
    pub members: Vec<Field>,
    group_type: GroupType,
    attributes: ElementAttributes,
}

impl TryFrom<&Vec<OwnedAttribute>> for Group {
    type Error = SyntacticError;
    fn try_from(attributes: &Vec<OwnedAttribute>) -> Result<Self, Self::Error> {
        let mut name = None;
        let mut self_attributes = ElementAttributes::new();
        let mut group_type = None;
        let context = String::from("field");

        for attribute in attributes {
            let attribute_name = &attribute.name.local_name;
            let value = &attribute.value;

            match attribute_name.as_str() {
                "name" => name = Some(value.clone()),
                "type" => group_type = Some(GroupType::try_from(value)?),
                _ => self_attributes.try_apply(&attribute_name, &value, &context)?,
            }
        }

        /*
         * forces named groups
        let name = name.ok_or_else(|| SyntacticError::UnnamedElement {
            context: String::from("group must have a name"),
        })?;
        */
        let name = name.unwrap_or(String::from(""));

        let group_type = group_type.unwrap_or(GroupType::Row);

        Ok(Self {
            name,
            group_type,
            title: None,
            instructions: None,
            attributes: self_attributes,
            members: Vec::new(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum FieldType {
    Text,
    Number,
    Checkbox,
    File,
    Image,
    Select,
    MultiSelect,
    TextArea,
    Date,
    Email,
    Tel,
    Url,
    Grid,
}

impl TryFrom<&String> for FieldType {
    type Error = SyntacticError;
    fn try_from(s: &String) -> Result<FieldType, Self::Error> {
        match s.as_str() {
            "text" => Ok(FieldType::Text),
            "number" => Ok(FieldType::Number),
            "date" => Ok(FieldType::Date),
            "checkbox" => Ok(FieldType::Checkbox),
            "select" => Ok(FieldType::Select),
            "multi-select" => Ok(FieldType::MultiSelect),
            "file" => Ok(FieldType::File),
            "image" => Ok(FieldType::Image),
            "textarea" => Ok(FieldType::TextArea),
            "email" => Ok(FieldType::Email),
            "tel" => Ok(FieldType::Tel),
            "url" => Ok(FieldType::Url),
            "grid" => Ok(FieldType::Grid),
            _ => Err(SyntacticError::InvalidFieldType {
                invalid_type: s.clone(),
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Field {
    pub name: String,
    field_type: FieldType,
    pub instructions: Option<String>,
    pub label: Option<String>,
    length: u16,
    placeholder: Option<String>,
    attributes: ElementAttributes,
    rows: Vec<u16>,
    pub options: Vec<FieldOption>,
}

impl Field {
    fn parse_rows(s: &String) -> Result<Vec<u16>, SyntacticError> {
        let mut result = Vec::new();
        for cell in s.split(' ') {
            if let Ok(dim) = cell.parse::<u16>() {
                result.push(dim)
            } else {
                return Err(SyntacticError::InvalidAttribute {
                    attribute_name: String::from("rows"),
                    context: format!("could not parse the value of rows attribute: {}", s),
                });
            }
        }
        Ok(result)
    }
}

impl TryFrom<&Vec<OwnedAttribute>> for Field {
    type Error = SyntacticError;
    fn try_from(attributes: &Vec<OwnedAttribute>) -> Result<Self, Self::Error> {
        let mut name = None;
        let mut self_attributes = ElementAttributes::new();
        let mut field_type = None;
        let mut placeholder = None;
        let mut length = 0u16;
        let mut rows = Vec::with_capacity(0);
        let context = String::from("field; unrecognized attribute");

        for attribute in attributes {
            let attribute_name = &attribute.name.local_name;
            let value = &attribute.value;

            match attribute_name.as_str() {
                "name" => name = Some(value.clone()),
                "type" => field_type = Some(FieldType::try_from(value)?),
                "placeholder" => placeholder = Some(value.clone()),
                "rows" => rows = Field::parse_rows(value)?,
                "length" => {
                    length = value
                        .parse()
                        .map_err(|_e| SyntacticError::InvalidAttribute {
                            attribute_name: String::from("length"),
                            context: String::from("field; length should be a whole number"),
                        })?
                }
                _ => self_attributes.try_apply(&attribute_name, &value, &context)?,
            }
        }

        let name = name.ok_or_else(|| SyntacticError::UnnamedElement {
            context: String::from("field must have a name"),
        })?;

        let field_type = field_type.ok_or_else(|| SyntacticError::InvalidFieldType {
            invalid_type: String::from("fields must have a type"),
        })?;

        Ok(Self {
            name,
            field_type,
            instructions: None,
            length,
            rows,
            label: None,
            placeholder,
            attributes: self_attributes,
            options: Vec::with_capacity(0),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FieldOption {
    pub name: String,
    pub label: Option<String>,
    attributes: ElementAttributes,
}

impl TryFrom<&Vec<OwnedAttribute>> for FieldOption {
    type Error = SyntacticError;
    fn try_from(attributes: &Vec<OwnedAttribute>) -> Result<Self, Self::Error> {
        let mut name = None;
        let mut self_attributes = ElementAttributes::new();
        let context = String::from("field");

        for attribute in attributes {
            let attribute_name = &attribute.name.local_name;
            let value = &attribute.value;

            match attribute_name.as_str() {
                "name" => name = Some(value.clone()),
                "lang" => {}
                _ => self_attributes.try_apply(&attribute_name, &value, &context)?,
            }
        }

        let name = name.ok_or_else(|| SyntacticError::UnnamedElement {
            context: String::from("option must have a name"),
        })?;

        Ok(Self {
            name,
            label: None,
            attributes: self_attributes,
        })
    }
}
