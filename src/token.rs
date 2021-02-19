use pug_cli as pug;
use std::path::PathBuf;
use xml::reader::{self, EventReader, XmlEvent};
use xml::{attribute::OwnedAttribute, name::OwnedName};

#[derive(Debug)]
enum Token {
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
    Language {
        characters: String,
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
struct TokenStream {
    tokens: Vec<Token>,
    alternates: Vec<String>,
    characters: Option<String>,
    lang: Option<String>,
}

impl TokenStream {
    fn from_readable_xml(source: impl std::io::Read) -> Result<TokenStream, xml::reader::Error> {
        let event_reader = EventReader::new(source);
        let mut token_stream = TokenStream {
            tokens: Vec::new(),
            alternates: Vec::new(),
            characters: None,
            lang: None,
        };

        for event in event_reader {
            token_stream.dispatch_event(event?);
        }

        return Ok(token_stream);
    }

    fn from_file(source: impl Into<PathBuf>) -> Result<TokenStream, Box<dyn std::error::Error>> {
        let pug_options = pug::PugOptions::new().doctype("xml".into());
        let xml = pug::evaluate_with_options(source, pug_options)?;
        return Ok(Self::from_readable_xml(xml.as_bytes())?);
    }

    fn on_start(&mut self, name: OwnedName, attributes: Vec<OwnedAttribute>) {
        match name.local_name.as_str() {
            "category" | "description" | "dir-description" | "meta-description" | "title"
            | "label" | "keywords" => {
                self.lang = attributes
                    .into_iter()
                    .find(|a| &a.name.local_name == "lang")
                    .map(|a| a.value)
            }
            "link" | "script" | "style" | "index" | "language" => {}
            "option" => self.tokens.push(Token::Option { attributes }),
            "field" => self.tokens.push(Token::Field { attributes }),
            "group" => self.tokens.push(Token::Group { attributes }),
            "section" => self.tokens.push(Token::Section { attributes }),

            _ => {} // TODO error
        }
    }
    fn on_end(&mut self, name: OwnedName) {
        let lang = self.lang.take();
        let characters = self.characters.take().unwrap_or_default();
        let token = match name.local_name.as_str() {
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
            "language" => Token::Language { characters },
            "option" => Token::OptionEnd,
            "field" => Token::FieldEnd,
            "group" => Token::GroupEnd,
            "section" => Token::SectionEnd,
            _ => Token::None,
        };

        self.tokens.push(token);
    }

    fn dispatch_event(&mut self, event: XmlEvent) {
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
    fn descriptions() {
        let ts = TokenStream::from_file("./resources/descriptions.pug").unwrap();
        println!("{:?}", ts);
    }
}
