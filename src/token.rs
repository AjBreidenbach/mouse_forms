use pug_cli as pug;
use std::path::PathBuf;
use xml::reader::{self, EventReader, XmlEvent};
use xml::{attribute::OwnedAttribute, name::OwnedName};

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

#[derive(Debug)]
pub enum Token {
    Category {
        characters: String,
        lang: Option<String>,
    },
    Description {
        characters: String,
        lang: Option<String>,
    },
    Index {
        position: u16,
    },
    DirDescription {
        characters: String,
        lang: Option<String>,
    },
    MetaDescription {
        characters: String,
        lang: Option<String>,
    },
    Instructions {
        characters: String,
        lang: Option<String>,
    },
    Title {
        characters: String,
        lang: Option<String>,
    },
    Label {
        characters: String,
        lang: Option<String>,
    },
    Link {
        characters: String,
    },
    Keywords {
        characters: String,
        lang: Option<String>,
    },
    Script {
        characters: String,
    },
    Style {
        characters: String,
    },
    Option {
        attributes: Vec<OwnedAttribute>,
    },
    Field {
        attributes: Vec<OwnedAttribute>,
    },

    Group {
        attributes: Vec<OwnedAttribute>,
    },
    Section {
        attributes: Vec<OwnedAttribute>,
    },
    OptionEnd,
    FieldEnd,
    GroupEnd,
    SectionEnd,
    Unlisted,
    None,
}

#[derive(Debug)]
pub struct TokenBuffer {
    pub tokens: Vec<Token>,
    pub alternates: Vec<String>,
    characters: Option<String>,
    // lang refers to lang attribute
    lang: Option<String>,
    instructions: Option<String>,
    // refers to default language of this form
    pub language: Option<String>,
}

impl TokenBuffer {
    pub fn from_readable_xml(
        source: impl std::io::Read,
    ) -> Result<TokenBuffer, xml::reader::Error> {
        let event_reader = EventReader::new(source);
        let mut token_stream = TokenBuffer {
            tokens: Vec::new(),
            alternates: Vec::new(),
            characters: None,
            lang: None,
            instructions: None,
            language: None,
        };

        for event in event_reader {
            token_stream.dispatch_event(event?);
        }

        return Ok(token_stream);
    }

    pub fn from_file(
        source: impl Into<PathBuf>,
    ) -> Result<TokenBuffer, Box<dyn std::error::Error>> {
        let pug_options = pug::PugOptions::new().doctype("xml".into());
        let xml = pug::evaluate_with_options(source, pug_options)?;
        return Ok(Self::from_readable_xml(xml.as_bytes())?);
    }

    pub fn from_file_with_obj(
        source: impl Into<PathBuf>,
        object: String,
    ) -> Result<TokenBuffer, Box<dyn std::error::Error>> {
        let pug_options = pug::PugOptions::new()
            .doctype("xml".into())
            .with_object(object);
        let xml = pug::evaluate_with_options(source, pug_options)?;
        return Ok(Self::from_readable_xml(xml.as_bytes())?);
    }

    fn set_lang(&mut self, attributes: Vec<OwnedAttribute>) {
        self.lang = attributes
            .into_iter()
            .find(|a| &a.name.local_name == "lang")
            .map(|a| a.value)
    }

    fn on_start(&mut self, name: OwnedName, attributes: Vec<OwnedAttribute>) {
        match name.local_name.as_str() {
            "category" | "description" | "dir-description" | "meta-description" | "title"
            | "label" | "keywords" => self.set_lang(attributes),
            "link" | "script" | "style" | "index" => {}
            "unlisted" => self.tokens.push(Token::Unlisted),
            "option" => self.tokens.push(Token::Option { attributes }),
            "field" => self.tokens.push(Token::Field { attributes }),
            "group" => self.tokens.push(Token::Group { attributes }),
            "section" => self.tokens.push(Token::Section { attributes }),
            "instructions" => {
                self.set_lang(attributes);
                self.instructions = Some(String::new());
            }

            _ => {} // TODO error
        }
    }
    fn on_end(&mut self, name: OwnedName) {
        let lang = self
            .lang
            .take()
            .or_else(|| self.language.clone())
            .map(|lang| if lang == "*" { None } else { Some(lang) })
            .flatten();

        let mut characters = self.characters.take().unwrap_or_default();
        if let Some(instructions) = self.instructions.take() {
            characters = instructions
        }

        let token = match name.local_name.as_str() {
            "language" => {
                self.language = Some(characters);
                Token::None
            }
            "category" => Token::Category { characters, lang },
            "description" => Token::Description { characters, lang },
            "dir-description" => Token::DirDescription { characters, lang },
            "meta-description" => Token::MetaDescription { characters, lang },
            "title" => Token::Title { characters, lang },
            "label" => Token::Label { characters, lang },
            "keywords" => Token::Keywords { characters, lang },
            "link" => Token::Link { characters },
            "script" => Token::Script { characters },
            "style" => Token::Style { characters },
            // TODO make this into a proper error?
            "index" => Token::Index {
                position: characters.parse().unwrap_or_default(),
            },
            "instructions" => Token::Instructions { characters, lang },
            "option" => Token::OptionEnd,
            "field" => Token::FieldEnd,
            "group" => Token::GroupEnd,
            "section" => Token::SectionEnd,
            "alternates" => {
                self.alternates = characters
                    .split(char::is_whitespace)
                    .map(String::from)
                    .collect();
                Token::None
            }
            _ => Token::None,
        };

        match token {
            Token::None => {}
            _ => self.tokens.push(token),
        }
    }

    fn dispatch_event(&mut self, event: XmlEvent) {
        if let Some(mut instructions) = self.instructions.take() {
            let mut resume = false;
            if let XmlEvent::EndElement { name } = &event {
                if name.local_name == "instructions" {
                    resume = true;
                }
            }
            if resume {
                self.instructions = Some(instructions);
            } else {
                instructions.push_str(&stringify_xml_event(event));
                self.instructions = Some(instructions);
                return;
            }
        }
        match event {
            XmlEvent::StartElement {
                name,
                attributes,
                namespace,
            } => self.on_start(name, attributes),
            XmlEvent::EndElement { name } => self.on_end(name),
            XmlEvent::Characters(characters) => self.characters = Some(characters),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_descriptions() {
        let ts = TokenBuffer::from_file("./resources/descriptions.pug").unwrap();
        println!("{:?}", ts);
    }

    #[test]
    fn tokens_form_instructions() {
        let ts = TokenBuffer::from_file("./resources/form-instructions.pug").unwrap();
        println!("{:?}", ts);
    }

    #[test]
    fn tokens_foreigner_arrival() {
        let ts =
            TokenBuffer::from_file("./resources/foreigner-arrival-notification.mf.pug").unwrap();
        println!("{:?}", ts);
    }
}
