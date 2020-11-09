pub extern crate pug_cli;
extern crate serde;
extern crate serde_yaml;
extern crate xml;

pub use pug_cli as pug;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::error;
use std::fmt;
use std::fs::File;
use std::io::{self, prelude::*, Read};
use std::path::PathBuf;
use xml::reader::{self, EventReader, XmlEvent};

fn stringify_xml_event(xml_event: XmlEvent) -> String {
    match xml_event {
        XmlEvent::StartElement {
            name,
            attributes,
            namespace,
        } => format!(
            "<{}{}>",
            name.local_name,
            attributes
                .into_iter()
                .fold(String::with_capacity(0), |acc, attribute| {
                    format!(
                        "{} {}=\"{}\"",
                        acc, attribute.name.local_name, attribute.value
                    )
                })
        ),
        XmlEvent::EndElement { name } => format!("</{}>", name.local_name),
        XmlEvent::Characters(characters) => characters,
        _ => String::with_capacity(0),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Form {
    title: Option<String>,
    unlisted: bool,
    description: Option<String>,
    embedded_script: Option<String>,
    category: Option<String>,
    index: u32,
    stylesheet: Option<String>,
    sections: Vec<FormSection>,
    language: Option<String>,
}

impl Form {
    fn new() -> Self {
        Form {
            title: None,
            unlisted: false,
            description: None,
            category: None,
            index: std::u32::MAX,
            embedded_script: None,
            stylesheet: None,
            sections: vec![],
            language: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FormSection {
    name: String,
    title: Option<String>,
    instructions: Option<String>,
    elements: Vec<FormElement>,
    attributes: ElementAttributes,
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
        attribute_name: String,
        value: String,
        context: &String,
    ) -> Result<(), SyntacticError> {
        match attribute_name.as_str() {
            "requires" => self.requires = Some(value),
            "optional" => self.optional = true,
            "optional-if" => self.optional_if = Some(value),
            "class" => self.class = Some(value),
            _ => {
                return Err(SyntacticError::InvalidAttribute {
                    attribute_name,
                    context: context.clone(),
                })
            }
        }
        Ok(())
    }
}
impl TryFrom<Vec<OwnedAttribute>> for FormSection {
    type Error = SyntacticError;
    fn try_from(attributes: Vec<OwnedAttribute>) -> Result<Self, Self::Error> {
        let mut name = None;
        let mut self_attributes = ElementAttributes::new();
        let context = String::from("section; attribute is unrecognized");

        for attribute in attributes {
            let attribute_name = attribute.name.local_name;
            let value = attribute.value;

            match attribute_name.as_str() {
                "name" => name = Some(value),
                _ => self_attributes.try_apply(attribute_name, value, &context)?,
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
enum FormElement {
    Group(FormGroup),
    Field(FormField),
}

#[derive(Serialize, Deserialize, Debug)]
enum GroupType {
    Row,
    Subsection,
}

impl TryFrom<String> for GroupType {
    type Error = SyntacticError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "row" => Ok(GroupType::Row),
            "subsection" => Ok(GroupType::Subsection),
            "" => Ok(GroupType::Row),
            _ => Err(SyntacticError::InvalidGroupType { invalid_type: s }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FormGroup {
    name: String,
    title: Option<String>,
    members: Vec<FormField>,
    group_type: GroupType,
    attributes: ElementAttributes,
}

impl TryFrom<Vec<OwnedAttribute>> for FormGroup {
    type Error = SyntacticError;
    fn try_from(attributes: Vec<OwnedAttribute>) -> Result<Self, Self::Error> {
        let mut name = None;
        let mut self_attributes = ElementAttributes::new();
        let mut group_type = None;
        let context = String::from("field");

        for attribute in attributes {
            let attribute_name = attribute.name.local_name;
            let value = attribute.value;

            match attribute_name.as_str() {
                "name" => name = Some(value),
                "type" => group_type = Some(GroupType::try_from(value)?),
                _ => self_attributes.try_apply(attribute_name, value, &context)?,
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
}

impl TryFrom<String> for FieldType {
    type Error = SyntacticError;
    fn try_from(s: String) -> Result<FieldType, Self::Error> {
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
            _ => Err(SyntacticError::InvalidFieldType { invalid_type: s }),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FormField {
    name: String,
    field_type: FieldType,
    instructions: Option<String>,
    label: Option<String>,
    length: u16,
    placeholder: Option<String>,
    attributes: ElementAttributes,
    options: Vec<FieldOption>,
}

impl TryFrom<Vec<OwnedAttribute>> for FormField {
    type Error = SyntacticError;
    fn try_from(attributes: Vec<OwnedAttribute>) -> Result<Self, Self::Error> {
        let mut name = None;
        let mut self_attributes = ElementAttributes::new();
        let mut field_type = None;
        let mut placeholder = None;
        let mut length = 0u16;
        let context = String::from("field; unrecognized attribute");

        for attribute in attributes {
            let attribute_name = attribute.name.local_name;
            let value = attribute.value;

            match attribute_name.as_str() {
                "name" => name = Some(value),
                "type" => field_type = Some(FieldType::try_from(value)?),
                "placeholder" => placeholder = Some(value),
                "length" => {
                    length = value
                        .parse()
                        .map_err(|_e| SyntacticError::InvalidAttribute {
                            attribute_name: String::from("length"),
                            context: String::from("field; length should be a whole number"),
                        })?
                }
                _ => self_attributes.try_apply(attribute_name, value, &context)?,
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
            label: None,
            placeholder,
            attributes: self_attributes,
            options: Vec::with_capacity(0),
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct FieldOption {
    name: String,
    label: Option<String>,
    attributes: ElementAttributes,
}

impl TryFrom<Vec<OwnedAttribute>> for FieldOption {
    type Error = SyntacticError;
    fn try_from(attributes: Vec<OwnedAttribute>) -> Result<Self, Self::Error> {
        let mut name = None;
        let mut self_attributes = ElementAttributes::new();
        let context = String::from("field");

        for attribute in attributes {
            let attribute_name = attribute.name.local_name;
            let value = attribute.value;

            match attribute_name.as_str() {
                "name" => name = Some(value),
                _ => self_attributes.try_apply(attribute_name, value, &context)?,
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

#[derive(Debug)]
struct FormParser {
    form: Form,
    current_instructions: Option<String>,
    current_section: Option<FormSection>,
    current_group: Option<FormGroup>,
    current_field: Option<FormField>,
    current_option: Option<FieldOption>,
    characters: String,
    path: Vec<String>,
}

use xml::{attribute::OwnedAttribute, name::OwnedName};
impl FormParser {
    fn new() -> Self {
        Self {
            form: Form::new(),
            current_instructions: None,
            current_section: None,
            current_group: None,
            current_field: None,
            current_option: None,
            characters: String::new(),
            path: Vec::new(),
        }
    }

    fn start_event(
        mut self,
        name: OwnedName,
        attributes: Vec<OwnedAttribute>,
    ) -> Result<Self, SyntacticError> {
        let name = name.local_name;

        match name.as_str() {
            "section" => {
                let section = FormSection::try_from(attributes)?;
                self.current_section = Some(section);
            }
            "field" => {
                if let Some(field) = self.current_field {
                    return Err(SyntacticError::ImproperNesting {
                        context: format!("field {} should not contain another field", field.name),
                    });
                }

                let field = FormField::try_from(attributes)?;
                self.current_field = Some(field);
            }
            "instructions" => self.current_instructions = Some(String::new()),
            "unlisted" => self.form.unlisted = true,
            "group" => {
                let group = FormGroup::try_from(attributes)?;
                self.current_group = Some(group);
            }
            "option" => {
                if let Some(option) = self.current_option {
                    return Err(SyntacticError::ImproperNesting {
                        context: format!(
                            "option {} should not contain another option",
                            option.name
                        ),
                    });
                }
                let option = FieldOption::try_from(attributes)?;
                self.current_option = Some(option);
            }
            _ => (),
        }
        self.path.push(name);
        Ok(self)
    }

    fn end_event(mut self, name: OwnedName) -> Result<Self, SyntacticError> {
        let name = name.local_name;
        if self.path.last() != Some(&name) {
            return Err(SyntacticError::MismatchedTags {
                open_tag: self.path.last().map(|o| o.clone()),
                closing_tag: name,
            });
        } else {
            self.path.pop();
        }

        match name.as_str() {
            "title" => {
                if let Some(ref mut group) = self.current_group {
                    group.title = Some(self.characters);
                } else if let Some(ref mut section) = self.current_section {
                    section.title = Some(self.characters);
                } else {
                    self.form.title = Some(self.characters);
                }
                self.characters = String::new();
            }
            "description" => {
                self.form.description = Some(self.characters);
                self.characters = String::new();
            }
            "language" => {
                self.form.language = Some(self.characters);
                self.characters = String::new();
            }
            "category" => {
                self.form.category = Some(self.characters);
                self.characters = String::new()
            }
            "index" => {
                self.form.index = self.characters.parse().unwrap_or(std::u32::MAX);
                self.characters = String::new()
            }

            "script" => {
                self.form.embedded_script = Some(self.characters);
                self.characters = String::new();
            }
            "style" => {
                self.form.stylesheet = Some(self.characters);
                self.characters = String::new();
            }
            // TODO add error handling
            "label" => {
                if let Some(ref mut option) = self.current_option {
                    option.label = Some(self.characters);
                } else if let Some(ref mut field) = self.current_field {
                    field.label = Some(self.characters);
                } else {
                    return Err(SyntacticError::OrphanElement {
                        context: format!(
                            "could not match label \"{}\" to a parent",
                            self.characters
                        ),
                    });
                }
                self.characters = String::new();
            }
            //combine label and title
            "section" => {
                if let Some(section) = self.current_section.take() {
                    self.form.sections.push(section);
                } else {
                    panic!("code blue monkey")
                }
            }
            "field" => {
                if let Some(mut field) = self.current_field.take() {
                    if self.characters.len() > 0 {
                        field.label = Some(field.label.unwrap_or(self.characters));
                        self.characters = String::new();
                    }
                    if let Some(ref mut group) = self.current_group {
                        group.members.push(field);
                    } else if let Some(ref mut section) = self.current_section {
                        section.elements.push(FormElement::Field(field));
                    } else {
                        return Err(SyntacticError::OrphanElement {
                            context: format!("field {} has no parent", field.name),
                        });
                    }
                }
            }
            "group" => {
                if let Some(group) = self.current_group.take() {
                    if let Some(ref mut section) = self.current_section {
                        section.elements.push(FormElement::Group(group));
                    } else {
                        return Err(SyntacticError::OrphanElement {
                            context: format!("group {} has no parent", group.name),
                        });
                    }
                }
            }
            "option" => {
                if let Some(mut option) = self.current_option.take() {
                    option.label = Some(option.label.unwrap_or(self.characters));
                    if let Some(ref mut field) = self.current_field {
                        field.options.push(option);
                    } else {
                        return Err(SyntacticError::OrphanElement {
                            context: format!("option {} has no parent", option.name),
                        });
                    }
                }
                self.characters = String::new();
            }
            _ => {}
        }

        Ok(self)
    }

    fn try_apply_event(mut self, event: XmlEvent) -> Result<Self, SyntacticError> {
        if let Some(mut instructions) = self.current_instructions {
            if let XmlEvent::EndElement { name } = &event {
                if name.local_name == "instructions" {
                    if let Some(ref mut field) = self.current_field {
                        field.instructions = Some(instructions)
                    } else if let Some(ref mut section) = self.current_section {
                        section.instructions = Some(instructions);
                    } else {
                        return Err(SyntacticError::OrphanElement {
                            context: String::from("instructions have no section parent"),
                        });
                    }
                    self.path.pop();
                    self.current_instructions = None;
                } else {
                    instructions.push_str(&stringify_xml_event(event));
                    self.current_instructions = Some(instructions);
                }
            } else {
                instructions.push_str(&stringify_xml_event(event));
                self.current_instructions = Some(instructions);
            }
            return Ok(self);
        }
        match event {
            XmlEvent::StartElement {
                name,
                attributes,
                namespace: _,
            } => self.start_event(name, attributes),
            XmlEvent::EndElement { name } => self.end_event(name),
            XmlEvent::Characters(c) => {
                self.characters.push_str(&c);
                Ok(self)
            }

            _ => Ok(self),
        }
    }
}

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

impl error::Error for SyntacticError {}

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

#[derive(Debug)]
pub enum FormParserError {
    Io(io::Error),
    Xml(reader::Error),
    Syntax(SyntacticError),
}

impl fmt::Display for FormParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            FormParserError::Io(io_error) => write!(f, "{}", io_error),
            FormParserError::Xml(reader_error) => write!(f, "{}", reader_error),
            FormParserError::Syntax(syntactic_error) => write!(f, "{}", syntactic_error),
            _ => write!(f, "syntax error"),
        }
    }
}

impl error::Error for FormParserError {}

#[derive(Debug)]
pub enum MouseFormsError {
    FormParser(FormParserError),
    Pug(pug::CompileError),
}

impl fmt::Display for MouseFormsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            Self::FormParser(parser_error) => write!(f, "{}", parser_error),
            Self::Pug(pug_error) => write!(f, "{}", pug_error),
        }
    }
}

impl error::Error for MouseFormsError {}

type FormParserResult = Result<Form, FormParserError>;

impl<R: Read> TryFrom<EventReader<R>> for Form {
    type Error = FormParserError;

    fn try_from(event_reader: EventReader<R>) -> FormParserResult {
        let mut parser = FormParser::new();
        for (i, event) in event_reader.into_iter().enumerate() {
            let event = event.map_err(|e| FormParserError::Xml(e))?;
            //eprintln!("{} {:?}", i, event);
            parser = parser
                .try_apply_event(event)
                .map_err(|e| FormParserError::Syntax(e))?;
        }
        Ok(parser.form)
    }
}

impl TryFrom<PathBuf> for Form {
    type Error = FormParserError;

    fn try_from(buf: PathBuf) -> FormParserResult {
        let file = File::open(buf).map_err(|e| FormParserError::Io(e))?;
        let event_reader = EventReader::new(file);

        Form::try_from(event_reader)
    }
}

impl TryFrom<String> for Form {
    type Error = FormParserError;

    fn try_from(source: String) -> FormParserResult {
        let event_reader = EventReader::from_str(&source);
        Form::try_from(event_reader)
    }
}
pub fn compile_to_json_str(file: impl Into<PathBuf>) -> Result<String, MouseFormsError> {
    let xml = pug::evaluate(file).map_err(|e| MouseFormsError::Pug(e))?;
    let mouse_form = Form::try_from(xml).map_err(|e| MouseFormsError::FormParser(e))?;
    let j = serde_json::to_string(&mouse_form).unwrap();
    Ok(j)
}

pub fn compile_to_json_str_with_obj(
    file: impl Into<PathBuf>,
    object: String,
) -> Result<String, MouseFormsError> {
    let pug_options = pug::PugOptions::new().with_object(object);
    let xml = pug::evaluate_with_options(file, pug_options).map_err(|e| MouseFormsError::Pug(e))?;
    let mouse_form = Form::try_from(xml).map_err(|e| MouseFormsError::FormParser(e))?;
    let j = serde_json::to_string(&mouse_form).unwrap();
    Ok(j)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn do_a_file(pug: &str) -> Result<(), Box<dyn error::Error>> {
        let xml = pug::evaluate(pug)?;
        let mouse_form = Form::try_from(xml)?;
        println!("{}", serde_yaml::to_string(&mouse_form)?);
        Ok(())
    }

    #[test]
    fn form_instructions() {
        do_a_file("resources/form-instructions.pug").unwrap();
    }

    #[test]
    fn placeholder() {
        do_a_file("resources/placeholder.pug").unwrap();
    }

    #[test]
    fn length() {
        do_a_file("resources/length.pug").unwrap();
    }

    /*
    #[test]
    fn it_works_again() {
        do_a_file("resources/select-group.mf.pug").unwrap();
    }
    */
}

// TODO
// error handling field at the wrong depth?
// do character pass to children?
