use std::from_str::FromStr;
use std::str;

#[deriving(Show)]
pub enum Ast {
    Literal(char),
    Cat(~Ast, ~Ast),
    Alt(~Ast, ~Ast),
    Rep(~Ast, Repeater, Greed),
}

#[deriving(Show, Eq)]
pub enum Repeater {
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
pub enum Greed {
    Greedy,
    Ungreedy,
}

impl Greed {
    pub fn is_greedy(&self) -> bool {
        match *self {
            Greedy => true,
            _ => false,
        }
    }
}

#[deriving(Show)]
pub enum Error {
    BadSyntax(~str),
}

#[deriving(Show)]
enum BuildAst {
    Ast(~Ast),
    Paren, // '('
    Bar, // '|'
}

impl BuildAst {
    fn paren(&self) -> bool {
        match *self {
            Paren => true,
            _ => false,
        }
    }

    fn bar(&self) -> bool {
        match *self {
            Bar => true,
            _ => false,
        }
    }
    
    fn unwrap(self) -> Result<~Ast, Error> {
        match self {
            Ast(x) => Ok(x),
            _ => Err(BadSyntax(~"TODO")),
        }
    }
}

struct Parser<'a> {
    chars: str::Chars<'a>,
    cur: Option<char>,
    stack: Vec<BuildAst>,
}

pub fn parse(s: &str) -> Result<~Ast, Error> {
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
                '(' => self.stack.push(Paren),
                ')' => {
                    let catfrom = try!(
                        self.pos_last(false, |x| x.paren() || x.bar()));
                    try!(self.concat(catfrom));

                    let altfrom = try!(self.pos_last(false, |x| x.paren()));
                    try!(self.alternate(altfrom));
                }
                '|' => {
                    let catfrom = try!(
                        self.pos_last(true, |x| x.paren() || x.bar()));
                    try!(self.concat(catfrom));

                    self.stack.push(Bar);
                }
                _ => try!(self.push_literal(c)),
            }
            self.next_char();
        }

        // Try to improve error handling. At this point, there should be
        // no remaining open parens.
        if self.stack.iter().any(|x| x.paren()) {
            return Err(BadSyntax(~"Unclosed paren."))
        }
        let catfrom = try!(self.pos_last(true, |x| x.bar()));
        try!(self.concat(catfrom));
        try!(self.alternate(0));

        assert!(self.stack.len() == 1);
        self.pop_ast()
    }

    fn next_char(&mut self) {
        self.cur = self.chars.next();
    }

    fn pop_ast(&mut self) -> Result<~Ast, Error> {
        match self.stack.pop().unwrap().unwrap() {
            Err(e) => Err(e),
            Ok(ast) => Ok(ast),
        }
    }

    fn push(&mut self, ast: ~Ast) {
        self.stack.push(Ast(ast))
    }

    fn push_repeater(&mut self, c: char) -> Result<(), Error> {
        if self.stack.len() == 0 {
            return Err(BadSyntax(~"Operator must be preceded by expression."))
        }
        let rep: Repeater = match from_char(c) {
            None => return Err(BadSyntax(~"Not a valid repeater operator.")),
            Some(r) => r,
        };

        match try!(self.pop_ast()) {
            ~Rep(ast, rep2, Greedy) => {
                if rep == ZeroOne {
                    self.push(~Rep(ast, rep2, Ungreedy))
                } else {
                    return Err(BadSyntax(~"Double repeat ops not supported."))
                }
            }
            ~Rep(_, _, Ungreedy) =>
                return Err(BadSyntax(~"Triple repeat ops not supported.")),
            ast => self.push(~Rep(ast, rep, Greedy)),
        }
        Ok(())
    }

    fn push_literal(&mut self, c: char) -> Result<(), Error> {
        self.stack.push(Ast(~Literal(c)));
        Ok(())
    }

    fn pos_last(&self, allow_start: bool, pred: |&BuildAst| -> bool)
               -> Result<uint, Error> {
        let from = match self.stack.iter().rev().position(pred) {
            Some(i) => i,
            None => {
                if allow_start {
                    self.stack.len()
                } else {
                    return Err(BadSyntax(~"No opening paren."))
                }
            }
        };
        // Adjust index since 'from' is for the reversed stack.
        // Also, don't include the '(' or '|'.
        Ok(self.stack.len() - from)
    }

    fn concat(&mut self, from: uint) -> Result<(), Error> {
        let ast = try!(self.build_from(from, Cat));
        self.push(ast);
        Ok(())
    }

    fn alternate(&mut self, mut from: uint) -> Result<(), Error> {
        // Unlike in the concatenation case, we want 'build_from' to continue
        // all the way to the opening left paren (so it will be popped off and
        // thrown away). But be careful with overflow---we can't count on the
        // open paren to be there.
        if from > 0 { from = from - 1}
        let ast = try!(self.build_from(from, Alt));
        self.push(ast);
        Ok(())
    }

    // build_from combines all AST elements starting at 'from' in the
    // parser's stack using 'mk' to combine them. If any such element is not an 
    // AST then it is popped off the stack and ignored.
    fn build_from(&mut self, from: uint, mk: |~Ast, ~Ast| -> Ast)
                 -> Result<~Ast, Error> {
        if from >= self.stack.len() {
            return Err(BadSyntax(~"Empty group or alternate not allowed."))
        }

        let mut combined = try!(self.pop_ast());
        let mut i = self.stack.len();
        while i > from {
            i = i - 1;
            match self.stack.pop().unwrap() {
                Ast(x) => combined = ~mk(x, combined),
                _ => {},
            }
        }
        Ok(combined)
    }
}

#[cfg(test)]
mod test {
    #[test]
    #[ignore]
    fn simple() {
        debug!("{}", super::parse("a|(b|(xyz))+"));
    }
}
