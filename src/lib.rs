pub extern crate pug_cli;
extern crate serde;
extern crate serde_json;
extern crate serde_yaml;

mod models;
mod parser;
mod token;

use models::Form;
use parser::Parser;
use token::TokenBuffer;

use std::path::PathBuf;

fn compile_with_token_buffer(ts: TokenBuffer) -> Result<Vec<Form>, Box<dyn std::error::Error>> {
    let mut ts = ts;
    let alternates = ts.alternates;
    ts.alternates = Vec::with_capacity(0);

    let mut forms = Vec::new();
    forms.push(Parser::new(&ts.tokens, ts.language.take()).parse()?);
    for alternate in alternates {
        forms.push(Parser::new(&ts.tokens, Some(alternate)).parse()?);
    }

    Ok(forms)
}

pub fn compile(source: impl Into<PathBuf>) -> Result<Vec<Form>, Box<dyn std::error::Error>> {
    let ts = TokenBuffer::from_file(source)?;
    compile_with_token_buffer(ts)
}

pub fn compile_with_obj(
    source: impl Into<PathBuf>,
    object: String,
) -> Result<Vec<Form>, Box<dyn std::error::Error>> {
    let ts = TokenBuffer::from_file_with_obj(source, object)?;
    compile_with_token_buffer(ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_foreigner_arrival() {
        let forms = compile("./resources/foreigner-arrival-notification.mf.pug").unwrap();
        println!("{}", serde_json::to_string(&forms).unwrap());
    }
}
