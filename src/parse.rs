use std::from_str::FromStr;
use std::str;

#[deriving(Show)]
enum Ast {
    Empty,
    Literal(char),
    Cat(~Ast, ~Ast),
    Alt(~Ast, ~Ast),
    Rep(~Ast, Repeater, Greed),
}

#[deriving(Show)]
enum Repeater {
    ZeroOne,
    ZeroMore,
    OneMore,
}

impl FromStr for Repeater {
    fn from_str(s: &str) -> Option<Repeater> {
        if s.len() != 1 { return None }
        match s.char_at(0) {
            '?' => Some(ZeroOne),
            '*' => Some(ZeroMore),
            '+' => Some(OneMore),
            _ => None,
        }
    }
}

fn from_char<T: FromStr>(c: char) -> Option<T> {
    from_str(str::from_char(c))
}

#[deriving(Show)]
enum Greed {
    Greedy,
    Ungreedy,
}

#[deriving(Show)]
enum Error {
    BadSyntax(~str),
}

#[deriving(Show)]
enum BuildAst {
    Ast(~Ast),
    Char(char),
}

impl BuildAst {
    fn char_is(&self, c: char) -> bool {
        match *self {
            Char(x) => c == x,
            _ => false,
        }
    }
    
    fn unwrap(self) -> Result<~Ast, Error> {
        match self {
            Ast(x) => Ok(x),
            Char(_) => Err(BadSyntax(~"TODO")),
        }
    }
}

struct Parser<'a> {
    chars: str::Chars<'a>,
    cur: Option<char>,
    stack: Vec<BuildAst>,
}

fn parse(s: &str) -> Result<~Ast, Error> {
    Parser {
        chars: s.chars(),
        cur: None,
        stack: vec!(),
    }.parse()
}

impl<'a> Parser<'a> {
    fn parse(&mut self) -> Result<~Ast, Error> {
        self.next_char();
        while !self.cur.is_none() {
            let c = self.cur.unwrap();
            match c {
                '?' | '*' | '+' => try!(self.push_repeater(c)),
                _ => try!(self.push_literal(c)),
            }
            self.next_char();
        }
        self.concat_from(0)
    }

    fn next_char(&mut self) {
        self.cur = self.chars.next();
    }

    fn push_repeater(&mut self, c: char) -> Result<(), Error> {
        match self.stack.len() {
            0 => Err(BadSyntax(~"Operator must be preceded by expression.")),
            _ => {
                let item = try!(self.stack.pop().unwrap().unwrap());
                match from_char(c) {
                    None => Err(BadSyntax(~"Not a valid repeater operator.")),
                    Some(r) => {
                        self.stack.push(Ast(~Rep(item, r, Greedy)));
                        Ok(())
                    }
                }
            }
        }
    }

    fn push_literal(&mut self, c: char) -> Result<(), Error> {
        self.stack.push(Ast(~Literal(c)));
        Ok(())
    }

    fn concat_from(&mut self, from: uint) -> Result<~Ast, Error> {
        assert!(from <= self.stack.len());
        match self.stack.len() - from {
            0 => return Ok(~Empty),
            1 => return Ok(try!(self.stack.pop().unwrap().unwrap())),
            _ => {},
        }
        let mut combined = try!(self.stack.pop().unwrap().unwrap());
        let mut i = self.stack.len();
        while i > from {
            let prev = try!(self.stack.pop().unwrap().unwrap());
            combined = ~Cat(prev, combined);
            i = i - 1;
        }
        Ok(combined)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn simple() {
        debug!("{}", super::parse("ab*"));
    }
}
