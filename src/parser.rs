use crate::models::*;
use crate::token::Token;
use std::convert::TryFrom;
pub struct Parser<'a> {
    language: Option<String>,
    tokens: &'a Vec<Token>,
    form: Form,
    current_section: Option<Section>,
    current_group: Option<Group>,
    current_field: Option<Field>,
    current_option: Option<FieldOption>,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: &'a Vec<Token>, language: Option<String>) -> Parser<'a> {
        let mut form = Form::new();
        form.language = language.clone();
        Parser {
            tokens,
            language,
            form,
            current_section: None,
            current_group: None,
            current_field: None,
            current_option: None,
        }
    }

    fn lang_matches(&self, lang: &Option<String>) -> bool {
        lang.is_none() || self.language == *lang
    }

    pub fn parse(mut self) -> Result<Form, SyntacticError> {
        let mut skip_option = false;
        for token in self.tokens {
            match token {
                Token::None => {}
                Token::Unlisted => self.form.unlisted = true,
                Token::Category { characters, lang } => {
                    if self.lang_matches(lang) {
                        self.form.category = Some(characters.clone())
                    }
                }
                Token::Description { characters, lang } => {
                    if self.lang_matches(lang) {
                        self.form.dir_description = Some(characters.clone());
                        self.form.meta_description = Some(characters.clone());
                        self.form.description = Some(characters.clone());
                    }
                }
                Token::MetaDescription { characters, lang } => {
                    if self.lang_matches(lang) {
                        self.form.meta_description = Some(characters.clone());
                    }
                }
                Token::DirDescription { characters, lang } => {
                    if self.lang_matches(lang) {
                        self.form.dir_description = Some(characters.clone());
                    }
                }
                Token::Keywords { characters, lang } => {
                    if self.lang_matches(lang) {
                        self.form.keywords = Some(characters.clone());
                    }
                }

                Token::Instructions { characters, lang } => {
                    if self.lang_matches(lang) {
                        if let Some(ref mut field) = self.current_field {
                            field.instructions = Some(characters.clone())
                        } else if let Some(ref mut group) = self.current_group {
                            group.instructions = Some(characters.clone())
                        } else if let Some(ref mut section) = self.current_section {
                            section.instructions = Some(characters.clone())
                        } else {
                            self.form.instructions = Some(characters.clone())
                        }
                    }
                }

                Token::Label { characters, lang } => {
                    if self.lang_matches(lang) {
                        if let Some(ref mut option) = self.current_option {
                            option.label = Some(characters.clone())
                        } else if let Some(ref mut field) = self.current_field {
                            field.label = Some(characters.clone())
                        } else {
                            //TODO error
                        }
                    }
                }

                Token::ImplicitLabel { characters } => {
                    if let Some(ref mut option) = self.current_option {
                        option.label = Some(characters.clone())
                    } else if let Some(ref mut field) = self.current_field {
                        field.label = Some(characters.clone())
                    } else if let Some(ref mut group) = self.current_group {
                        group.title = Some(characters.clone())
                    } else if let Some(ref mut section) = self.current_section {
                        section.title = Some(characters.clone())
                    }
                }

                Token::Title { characters, lang } => {
                    if self.lang_matches(lang) {
                        if let Some(ref mut group) = self.current_group {
                            group.title = Some(characters.clone())
                        } else if let Some(ref mut section) = self.current_section {
                            section.title = Some(characters.clone())
                        } else {
                            self.form.title = Some(characters.clone())
                        }
                    }
                }

                Token::Section { attributes } => {
                    self.current_section = Some(Section::try_from(attributes)?)
                }
                Token::Group { attributes } => {
                    self.current_group = Some(Group::try_from(attributes)?)
                }
                Token::Field { attributes } => {
                    self.current_field = Some(Field::try_from(attributes)?)
                }
                Token::Option { attributes } => {
                    let lang = attributes
                        .iter()
                        .find(|a| a.name.local_name == "lang")
                        .map(|a| a.value.clone());
                    if self.lang_matches(&lang) {
                        self.current_option = Some(FieldOption::try_from(attributes)?)
                    } else {
                        skip_option = true;
                    }
                }

                Token::SectionEnd => {
                    self.form
                        .sections
                        .push(self.current_section.take().ok_or_else(|| {
                            SyntacticError::MismatchedTags {
                                open_tag: None,
                                closing_tag: String::from("section"),
                            }
                        })?)
                }
                Token::GroupEnd => {
                    let group = self.current_group.take().ok_or_else(|| {
                        SyntacticError::MismatchedTags {
                            open_tag: None,
                            closing_tag: String::from("group"),
                        }
                    })?;

                    if let Some(ref mut section) = self.current_section {
                        section.elements.push(FormElement::Group(group));
                    } else {
                        Err(SyntacticError::OrphanElement {
                            context: String::from("group found without a parent section"),
                        })?;
                    }
                }

                Token::FieldEnd => {
                    let field = self.current_field.take().ok_or_else(|| {
                        SyntacticError::MismatchedTags {
                            open_tag: None,
                            closing_tag: String::from("field"),
                        }
                    })?;
                    if let Some(ref mut group) = self.current_group {
                        group.members.push(field);
                    } else if let Some(ref mut section) = self.current_section {
                        section.elements.push(FormElement::Field(field));
                    } else {
                        Err(SyntacticError::OrphanElement {
                            context: String::from("field found without a parent section or group"),
                        })?;
                    }
                }

                Token::OptionEnd => {
                    if skip_option {
                        skip_option = false;
                        continue;
                    }
                    let option = self.current_option.take().ok_or_else(|| {
                        SyntacticError::MismatchedTags {
                            open_tag: None,
                            closing_tag: String::from("option"),
                        }
                    })?;

                    if let Some(ref mut field) = self.current_field {
                        //TODO check that field type is select
                        field.options.push(option);
                    } else {
                        Err(SyntacticError::OrphanElement {
                            context: String::from("option found without field parent"),
                        })?;
                    }
                }

                Token::Index { position } => self.form.index = *position,
                Token::Link { characters } => self.form.link = Some(characters.clone()),
                Token::Script { characters } => self.form.embedded_scripts.push(characters.clone()),
                Token::Style { characters } => self.form.stylesheet = Some(characters.clone()),
            }
        }
        Ok(self.form)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::TokenBuffer;

    #[test]
    fn parse_descriptions() {
        let ts = TokenBuffer::from_file("./resources/descriptions.pug").unwrap();
        let form = Parser::new(&ts.tokens, Some("en".into())).parse().unwrap();

        println!("{}", serde_yaml::to_string(&form).unwrap());
    }
    #[test]
    fn parse_foreigner_arrival() {
        let ts =
            TokenBuffer::from_file("./resources/foreigner-arrival-notification.mf.pug").unwrap();
        let form = Parser::new(&ts.tokens, Some("en".into())).parse().unwrap();

        println!("{}", serde_yaml::to_string(&form).unwrap());
    }
    #[test]
    fn parse_foreigner_data_change() {
        let ts = TokenBuffer::from_file("./resources/foreigner-data-change.mf.pug").unwrap();
        let form = Parser::new(&ts.tokens, Some("en".into())).parse().unwrap();

        println!("{}", serde_yaml::to_string(&form).unwrap());
    }
    #[test]
    fn parse_implicit_label() {
        let ts = TokenBuffer::from_file("./resources/implicit-label.pug").unwrap();
        let form = Parser::new(&ts.tokens, Some("en".into())).parse().unwrap();
        println!("{}", serde_yaml::to_string(&form).unwrap());
    }
}
