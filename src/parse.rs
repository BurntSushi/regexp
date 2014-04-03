use std::from_str::FromStr;
use std::str;

use super::{Error, ErrorKind, Bug, BadSyntax};

#[deriving(Show)]
pub enum Ast {
    Literal(char, bool),
    Dot(bool),
    Begin(bool),
    End(bool),
    Capture(uint, ~Ast),
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

    fn swap(self, swapped: bool) -> Greed {
        if !swapped { return self }
        match self {
            Greedy => Ungreedy,
            Ungreedy => Greedy,
        }
    }
}

#[deriving(Show)]
enum BuildAst {
    Ast(~Ast),
    Paren(Flags, uint), // '('
    Bar, // '|'
}

impl BuildAst {
    fn paren(&self) -> bool {
        match *self {
            Paren(_, _) => true,
            _ => false,
        }
    }

    fn flags(&self) -> Flags {
        match *self {
            Paren(flags, _) => flags,
            _ => fail!("Cannot get flags from {}", self),
        }
    }

    fn capture(&self) -> Option<uint> {
        match *self {
            Paren(_, 0) => None,
            Paren(_, c) => Some(c),
            _ => fail!("Cannot get flags from {}", self),
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
            _ => Err(Error {
                pos: 0,
                kind: Bug,
                msg: ~"Tried to unwrap non-AST item.",
            })
        }
    }
}

#[deriving(Show)]
struct Flags(uint);

#[deriving(Show)]
pub enum Flag {
    Empty = 0,
    CaseI = 1, // i
    Multi = 2, // m
    DotNL = 4, // s
    SwapGreed = 8, // U
}

impl Flags {
    fn is_set(&self, f2: Flag) -> bool {
        let Flags(f1) = *self;
        f1 & (f2 as uint) > 0
    }
}

impl BitAnd<Flag, Flags> for Flags {
    fn bitand(&self, rhs: &Flag) -> Flags {
        let Flags(f) = *self;
        Flags(f & ((*rhs) as uint))
    }
}

impl BitOr<Flag, Flags> for Flags {
    fn bitor(&self, rhs: &Flag) -> Flags {
        let Flags(f) = *self;
        Flags(f | ((*rhs) as uint))
    }
}

impl BitXor<Flags, Flags> for Flags {
    fn bitxor(&self, rhs: &Flags) -> Flags {
        let (Flags(f1), Flags(f2)) = (*self, *rhs);
        Flags(f1 ^ (f2 as uint))
    }
}

struct Parser<'a> {
    chars: Vec<char>,
    chari: uint,
    stack: Vec<BuildAst>,
    flags: Flags,
    caps: uint,
}

pub fn parse(s: &str) -> Result<~Ast, Error> {
    Parser {
        chars: s.chars().collect(),
        chari: 0,
        stack: vec!(),
        flags: Flags(Empty as uint),
        caps: 0,
    }.parse()
}

impl<'a> Parser<'a> {
    fn parse(&mut self) -> Result<~Ast, Error> {
        while self.chari < self.chars.len() {
            let c = self.cur();
            match c {
                '?' | '*' | '+' => try!(self.push_repeater(c)),
                '(' => {
                    if self.peek_is(1, '?') {
                        self.next_char();
                        self.next_char();
                        try!(self.parse_group_opts())
                    } else {
                        self.caps += 1;
                        self.stack.push(Paren(self.flags, self.caps))
                    }
                }
                ')' => {
                    let catfrom = try!(
                        self.pos_last(false, |x| x.paren() || x.bar()));
                    try!(self.concat(catfrom));

                    let altfrom = try!(self.pos_last(false, |x| x.paren()));
                    // Before we smush the alternates together and pop off the
                    // left paren, let's grab the old flags and see if we
                    // need a capture.
                    let (cap, oldflags) = {
                        let paren = self.stack.get(altfrom-1);
                        (paren.capture(), paren.flags())
                    };
                    try!(self.alternate(altfrom));
                    self.flags = oldflags;

                    // If this was a capture, pop what we just pushed in
                    // alternate and make it a capture.
                    if cap.is_some() {
                        let ast = try!(self.pop_ast());
                        self.push(~Capture(cap.unwrap(), ast));
                    }
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
            return self.err(BadSyntax, "Unclosed parenthesis.")
        }
        let catfrom = try!(self.pos_last(true, |x| x.bar()));
        try!(self.concat(catfrom));
        try!(self.alternate(0));

        assert!(self.stack.len() == 1);
        self.pop_ast()
    }

    fn next_char(&mut self) {
        self.chari += 1;
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
            return self.synerr(
                "A repeat operator must be preceded by a valid expression.")
        }
        let rep: Repeater = match from_char(c) {
            None => return self.err(Bug, "Not a valid repeater operator."),
            Some(r) => r,
        };

        match self.peek(1) {
            Some('*') | Some('+') =>
                return self.synerr(
                    "Double repeat operators are not supported."),
            _ => {},
        }
        let greed = {
            if self.peek_is(1, '?') {
                self.next_char();
                Ungreedy
            } else {
                Greedy
            }
        }.swap(self.flags.is_set(SwapGreed));

        // match try!(self.pop_ast()) { 
            // ~Rep(_, _, _) => 
                // return self.synerr( 
                    // "Double repeat operators are not supported."), 
            // ast => self.push(~Rep(ast, rep, greed)), 
        // } 
        let ast = try!(self.pop_ast());
        self.push(~Rep(ast, rep, greed));
        Ok(())
    }

    fn push_literal(&mut self, c: char) -> Result<(), Error> {
        match c {
            '.' => {
                let dotnl = self.flags.is_set(DotNL);
                self.push(~Dot(dotnl))
            }
            '^' => {
                let multi = self.flags.is_set(Multi);
                self.push(~Begin(multi))
            }
            '$' => {
                let multi = self.flags.is_set(Multi);
                self.push(~End(multi))
            }
            _ => {
                let casei = self.flags.is_set(CaseI);
                self.push(~Literal(c, casei))
            }
        }
        Ok(())
    }

    // Parses non-capture groups and options.
    // Assumes that '(?' has already been consumed.
    fn parse_group_opts(&mut self) -> Result<(), Error> {
        let start = self.chari;
        let mut flags = self.flags;
        let mut sign = 1;
        let mut saw_flag = false;
        while self.chari < self.chars.len() {
            match self.cur() {
                'i' => { flags = flags | CaseI;     saw_flag = true},
                'm' => { flags = flags | Multi;     saw_flag = true},
                's' => { flags = flags | DotNL;     saw_flag = true},
                'U' => { flags = flags | SwapGreed; saw_flag = true},
                '-' => {
                    if sign < 0 {
                        return self.synerr(format!(
                            "Cannot negate flags twice in '{}'.",
                            self.slice(start, self.chari + 1)))
                    }
                    sign = -1;
                    saw_flag = false;
                    flags = flags ^ flags;
                }
                ':' | ')' => {
                    if sign < 0 {
                        if !saw_flag {
                            return self.synerr(format!(
                                "A valid flag does not follow negation in '{}'",
                                self.slice(start, self.chari + 1)))
                        }
                        flags = flags ^ flags;
                    }
                    if self.cur() == ':' {
                        // Save the old flags with the opening paren.
                        self.stack.push(Paren(self.flags, 0));
                    }
                    self.flags = flags;
                    return Ok(())
                }
                _ => return self.synerr(format!(
                    "Unrecognized flag '{}'.", self.cur())),
            }
            self.next_char();
        }
        self.synerr(format!(
            "Invalid flags: '{}'", self.slice(start, self.chari)))
    }

    fn pos_last(&self, allow_start: bool, pred: |&BuildAst| -> bool)
               -> Result<uint, Error> {
        let from = match self.stack.iter().rev().position(pred) {
            Some(i) => i,
            None => {
                if allow_start {
                    self.stack.len()
                } else {
                    return self.synerr("No matching opening parenthesis.")
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
            return self.synerr("Empty group or alternate not allowed.")
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

    fn err<T>(&self, k: ErrorKind, msg: &str) -> Result<T, Error> {
        Err(Error {
            pos: self.chari,
            kind: k,
            msg: msg.to_owned(),
        })
    }

    fn synerr<T>(&self, msg: &str) -> Result<T, Error> {
        self.err(BadSyntax, msg)
    }

    fn peek(&self, offset: uint) -> Option<char> {
        if self.chari + offset >= self.chars.len() {
            return None
        }
        Some(*self.chars.get(self.chari + offset))
    }

    fn peek_is(&self, offset: uint, is: char) -> bool {
        self.peek(offset) == Some(is)
    }

    fn cur(&self) -> char {
        *self.chars.get(self.chari)
    }

    fn slice(&self, start: uint, end: uint) -> ~str {
        str::from_chars(self.chars.as_slice().slice(start, end))
    }
}

#[cfg(test)]
mod test {
    #[test]
    #[ignore]
    fn simple() {
        debug!("{}", super::parse("a(?i)nd"));
    }
}
