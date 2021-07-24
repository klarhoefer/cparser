
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::fs::File;


struct Tokenizer<'a> {
    line: &'a str,
}

impl<'a> Tokenizer<'a> {
    fn new(line: &'a str) -> Self {
        Tokenizer { line }
    }

}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.line.len() > 0 {
            for (i, c) in self.line.chars().enumerate() {
                match c {
                    ';' | ',' | '(' | ')' | '{' | '}' | '[' | ']' | '*' | '&' | ' ' | '\t' | '-' | '+' | '<' | '>' | ':' | '?' | '=' | '|' => {
                        let pos = if i == 0 {
                            1
                        } else {
                            i
                        };
                        let res = &self.line[..pos];
                        self.line = &self.line[pos..].trim_start();
                        return Some(res);
                    },
                    _ => (),
                }
            }
            let res = self.line;
            self.line = "";
            Some(res)
        } else {
            None
        }
    }
}

#[derive(Debug)]
enum Token {
    Reserved(String),
    Identifier(String),
    Operator(String),
}

fn parse<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.len() == 0 || trimmed.starts_with("#line") {
            continue;
        }

        if trimmed.starts_with("#pragma") {
            continue;
        }

        let tokenizer = Tokenizer::new(&trimmed);
        for token in tokenizer {
            let token = match token {
                "typedef" | "struct" | "union" | "for" | "while" | "if" | "extern" | "sizeof" | "enum" |
                    "void" | "int" | "double" | "float" | "unsigned" | "char" | "const" | "long" | "short" => Token::Reserved(token.into()),
                "*" | ";" | "!" | "(" | ")" | "{" | "}" | "[" | "]" | "," | "=" => Token::Operator(token.into()),
                _ => Token::Identifier(token.into()),
            };
            match token {
                Token::Identifier(s) => {
                    println!("\"{}\"", s);
                },
                _ => (),
            }
        }

    }

    Ok(())
}



fn main() {
    // file created with cl /P <header file>
    match parse("hdf5.i") {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("{:?}", e),
    }
}
