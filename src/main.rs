use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::fs::File;


struct Tokenizer<'a> {
    line: &'a str,
    text: &'a [u8],
    last: usize,
    pos: usize
}

fn is_whitespace(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

impl<'a> Tokenizer<'a> {

    fn new(line: &'a str) -> Self {
        let text = line.as_bytes();
        Tokenizer { line, text, last: 0, pos: 0 }
    }

    fn available(&self) -> bool {
        self.pos < self.text.len()
    }

    fn step(&mut self) {
        self.pos += 1;
    }

    fn step_n(&mut self, n: usize) {
        self.pos += n;
    }

    fn current(&self) -> Option<u8> {
        if self.available() {
            Some(self.text[self.pos])
        } else {
            None
        }
    }

    fn peek(&self) -> Option<u8> {
        if self.pos + 1 < self.text.len() {
            Some(self.text[self.pos + 1])
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(b) = self.current() {
            if is_whitespace(b) {
                self.step();
            } else {
                break;
            }
        }
    }

    fn slice(&self) -> &'a str {
        &self.line[self.last..self.pos]
    }

    fn identifier(&mut self) -> &'a str {
        while let Some(b) = self.current() {
            match b {
                b'_' | b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' => self.step(),
                _ => break,
            }
        }

        self.slice()
    }

    fn number(&mut self) -> &'a str {
        while let Some(b) = self.current() {
            match b {
                b'0'..=b'9' | b'.' => self.step(),
                _ => break,
            }
        }

        self.slice()
    }

    fn character(&mut self) -> &'a str {
        let mut mask = false;
        while let Some(b) = self.current() {
            self.step();

            match b {
                b'\\' => mask = true,
                _ if mask => mask = false,
                b'\'' => return self.slice(),
                _ => (),
            }
        }
        unreachable!()
    }

    fn string(&mut self) -> &'a str {
        let mut mask = false;
        while let Some(b) = self.current() {
            self.step();

            match b {
                b'\\' => mask = true,
                _ if mask => mask = false,
                b'\"' => return self.slice(),
                _ => (),
            }
        }
        unreachable!()
    }
}


impl<'a> Iterator for Tokenizer<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.skip_whitespace();

        self.last = self.pos;

        if let Some(b) = self.current() {
            self.step();

            match b {
                b';' | b',' | b'{' | b'}' | b'[' | b']' | b'(' | b')' | b'?' | b':' => return Some(self.slice()),
                b'.' => {
                    match (self.current(), self.peek()) {
                        (Some(b'.'), Some(b'.')) => self.step_n(2),
                        _ => (),
                    }
                    return Some(self.slice());
                },
                b'*' | b'^' | b'!' | b'=' | b'/' => {
                    match self.current() {
                        Some(b'=') => self.step(),
                        _ => (),
                    }
                    return Some(self.slice());
                },
                b'&' | b'|' | b'+' => {
                    match self.current() {
                        Some(b'=') => self.step(),
                        Some(b2) if b2 == b => self.step(),
                        _ => (),
                    }
                    return Some(self.slice());
                },
                b'<' | b'>' => {
                    match self.current() {
                        Some(b'=') => self.step(),
                        Some(b2) if b2 == b => {
                            self.step();
                            match self.current() {
                                Some(b'=') => self.step(),
                                _ => (),
                            }
                        },
                        _ => (),
                    }
                    return Some(self.slice());
                },
                b'-' => {
                    match self.current() {
                        Some(b'>') | Some(b'-') | Some(b'=') => self.step(),
                        _ => (),
                    }
                    return Some(self.slice());
                },
                b'_' | b'a'..=b'z' | b'A'..=b'Z' => return Some(self.identifier()),
                b'0'..=b'9' => return Some(self.number()),
                b'"' => return Some(self.string()),
                b'\'' => return Some(self.character()),
                _ => unreachable!(),
            }
        }
        None
    }
}

fn try_parse_struct(stmt: &[String]) {
    if stmt[0] == "struct" {
        let l = stmt.len();
        let mut p = 1;
        while p < l && stmt[p] != "{" {
            p += 1;
        }
        p += 1;

        while p < l {
            let mut i = p + 1;
            while i < l && stmt[i] != ";" {
                i += 1;
            }
            if i < l {
                println!("\ttype = {:?}", &stmt[p..i]);
            }
            p = i + 1;
        }
    }
}

fn parse_statement(stmt: &[String]) {
    let l = stmt.len();
    if l > 2 {
        if stmt[0] == "typedef" {
            let name = &stmt[l - 2];
            println!("name = {}", name);
            try_parse_struct(&stmt[1..l-2]);
        }
    }
}


fn parse<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);

    let mut statement = Vec::new();
    let mut balance = 0;
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.len() == 0 || trimmed.starts_with("#line") || trimmed.starts_with("#pragma") {
            continue;
        }

        let tokenizer = Tokenizer::new(&trimmed);
        for token in tokenizer {
            statement.push(token.to_string());
            match token {
                "{" | "[" | "(" => balance += 1,
                "}" | "]" | ")" => balance -= 1,
                ";" if balance == 0 => {
                    parse_statement(&statement);
                    // let stmt = statement.join(" ");
                    // println!("~ {}", stmt);
                    statement.clear();
                },
                _ => (),
            }
        }

    }

    Ok(())
}



fn main() {
    // file created with cl /P <header file>
    match parse(r"..\processEsie_HST4.i") {
        Ok(_) => println!("OK"),
        Err(e) => eprintln!("{:?}", e),
    }
}


#[cfg(test)]
mod tests {

    use super::Tokenizer;

    #[test]
    fn it_works() {
        let tokenizer = Tokenizer::new("abc def = == ! != xyz ... < << <<= > >> >>=");
        for token in tokenizer {
            println!("{}", token);
        }
    }
}
