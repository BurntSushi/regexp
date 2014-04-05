use std::char;
use std::cmp;
use std::from_str::FromStr;
use std::iter;
use std::mem;
use std::num;
use std::str;

use super::{Error, ErrorKind, Bug, BadSyntax};
use self::unicode::UNICODE_CLASSES;

mod unicode;

static MAX_REPEAT: uint = 1000;

#[deriving(Show, Clone)]
pub enum Ast {
    Nothing,
    Literal(char, bool),
    Dot(bool),
    Class(Vec<(char, char)>, bool, bool),
    Begin(bool),
    End(bool),
    WordBoundary(bool),
    Capture(uint, Option<~str>, ~Ast),
    Cat(~Ast, ~Ast),
    Alt(~Ast, ~Ast),
    Rep(~Ast, Repeater, Greed),
}

#[deriving(Show, Eq, Clone)]
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

#[deriving(Show, Clone)]
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
    Paren(Flags, uint, ~str), // '('
    Bar, // '|'
}

impl BuildAst {
    fn paren(&self) -> bool {
        match *self {
            Paren(_, _, _) => true,
            _ => false,
        }
    }

    fn flags(&self) -> Flags {
        match *self {
            Paren(flags, _, _) => flags,
            _ => fail!("Cannot get flags from {}", self),
        }
    }

    fn capture(&self) -> Option<uint> {
        match *self {
            Paren(_, 0, _) => None,
            Paren(_, c, _) => Some(c),
            _ => fail!("Cannot get capture group from {}", self),
        }
    }

    fn capture_name(&self) -> Option<~str> {
        match *self {
            Paren(_, 0, _) => None,
            Paren(_, _, ref name) => {
                if name.len() == 0 {
                    None
                } else {
                    Some(name.clone())
                }
            }
            _ => fail!("Cannot get capture name from {}", self),
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

fn combine_ranges(unordered: Vec<(char, char)>) -> Vec<(char, char)> {
    // This is currently O(n^2), but I think with sufficient cleverness,
    // it can be reduced to O(n) **if necessary**.
    let mut ordered: Vec<(char, char)> = Vec::with_capacity(unordered.len());
    for (us, ue) in unordered.move_iter() {
        let (mut us, mut ue) = (us, ue);
        assert!(us <= ue);
        let mut which: Option<uint> = None;
        for (i, &(os, oe)) in ordered.iter().enumerate() {
            if should_merge((us, ue), (os, oe)) {
                us = cmp::min(us, os);
                ue = cmp::max(ue, oe);
                which = Some(i);
                break
            }
        }
        match which {
            None => ordered.push((us, ue)),
            Some(i) => *ordered.get_mut(i) = (us, ue),
        }
    }
    ordered.sort();
    ordered
}

fn should_merge((a, b): (char, char), (x, y): (char, char)) -> bool {
    cmp::max(a, x) as u32 <= cmp::min(b, y) as u32 + 1
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
                '\\' => {
                    let ast = try!(self.parse_escape());
                    self.push(ast)
                }
                '{' => try!(self.parse_counted()),
                '[' => match self.try_parse_ascii() {
                    None => try!(self.parse_class()),
                    Some(class) => self.push(class),
                },
                '(' => {
                    if self.peek_is(1, '?') {
                        self.next_char();
                        self.next_char();
                        try!(self.parse_group_opts())
                    } else {
                        self.caps += 1;
                        self.stack.push(Paren(self.flags, self.caps, ~""))
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
                    let (cap, cap_name, oldflags) = {
                        let paren = self.stack.get(altfrom-1);
                        (paren.capture(), paren.capture_name(), paren.flags())
                    };
                    try!(self.alternate(altfrom));
                    self.flags = oldflags;

                    // If this was a capture, pop what we just pushed in
                    // alternate and make it a capture.
                    if cap.is_some() {
                        let ast = try!(self.pop_ast());
                        self.push(~Capture(cap.unwrap(), cap_name, ast));
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
        let ast = try!(self.pop_ast());
        let greed = self.get_next_greedy();
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

    // Parses all forms of character classes.
    // Assumes that '[' has already been consumed.
    fn parse_class(&mut self) -> Result<(), Error> {
        let start = self.chari;
        let negated = self.peek_is(1, '^');
        if negated { self.next_char() }
        let mut ranges: Vec<(char, char)> = vec!();
        let mut alts: Vec<~Ast> = vec!();

        while self.peek_is(1, '-') {
            self.next_char();
            ranges.push(('-', '-'))
        }
        if self.peek_is(1, ']') {
            self.next_char();
            ranges.push((']', ']'))
        }
        self.next_char();
        while self.chari < self.chars.len() {
            let mut c = self.cur();
            match c {
                '[' =>
                    match self.try_parse_ascii() {
                        Some(~Class(asciis, neg, casei)) => {
                            alts.push(~Class(asciis, neg ^ negated, casei));
                            self.next_char();
                            continue
                        }
                        Some(ast) => return self.err(Bug, format!(
                            "Expected Class AST but got '{}'", ast)),
                        // Just drop down and try to add as a regular character.
                        None => {},
                    },
                '\\' => {
                    match try!(self.parse_escape()) {
                        ~Class(asciis, neg, casei) => {
                            alts.push(~Class(asciis, neg ^ negated, casei));
                            self.next_char();
                            continue
                        }
                        ~Literal(c2, _) => c = c2, // process below
                        ~Begin(_) | ~End(_) | ~WordBoundary(_) =>
                            return self.synerr(
                                "\\A, \\z, \\b and \\B are not valid escape \
                                 sequences inside a character class."),
                        ast => return self.err(Bug, format!(
                            "Unexpected AST item '{}'", ast)),
                    }
                }
                _ => {},
            }
            match c {
                ']' => {
                    let mut ast = ~Nothing;
                    if ranges.len() > 0 {
                        let casei = self.flags.is_set(CaseI);
                        ast = ~Class(combine_ranges(ranges), negated, casei);
                    }
                    for alt in alts.move_iter() {
                        ast = ~Alt(alt, ast)
                    }
                    self.push(ast);
                    return Ok(())
                }
                c => {
                    if self.peek_is(1, '-') && !self.peek_is(2, ']') {
                        self.next_char(); self.next_char();
                        let c2 = self.cur();
                        if c2 < c {
                            return self.synerr(format!(
                                "Invalid character class range '{}-{}'", c, c2))
                        }
                        ranges.push((c, self.cur()))
                    } else {
                        ranges.push((c, c))
                    }
                }
            }
            self.next_char()
        }
        self.synerr(format!(
            "Could not find closing ']' for character class starting \
             as position {}.", start))
    }

    // Tries to parse an ASCII character class of the form [:name:].
    // If successful, returns an AST character class corresponding to name.
    // If unsuccessful, no state is changed and None is returned.
    // Assumes that '[' has been parsed.
    fn try_parse_ascii(&mut self) -> Option<~Ast> {
        if !self.peek_is(1, ':') {
            return None
        }
        let closer =
            match self.pos(']') {
                Some(i) => i,
                None => return None,
            };
        if *self.chars.get(closer-1) != ':' {
            return None
        }
        if closer - self.chari <= 3 {
            return None
        }
        let negated = self.peek_is(2, '^');
        let mut name_start = self.chari + 2;
        if negated { name_start += 1 }
        let name = self.slice(name_start, closer - 1);
        match find_class(ASCII_CLASSES, name) {
            None => None,
            Some(ranges) => {
                let casei = self.flags.is_set(CaseI);
                self.chari = closer;
                Some(~Class(combine_ranges(ranges), negated, casei))
            }
        }
    }

    // Parses counted repetition. Supports:
    // {n}, {n,}, {n,m}, {n}?, {n,}? and {n,m}?
    // Assumes that '{' has already been consumed.
    fn parse_counted(&mut self) -> Result<(), Error> {
        // Scan until the closing '}' and grab the stuff in {}.
        let start = self.chari;
        let closer =
            match self.pos('}') {
                Some(i) => i,
                None => return self.synerr(format!(
                    "No closing brace for counted repetition starting at \
                     position {}.", start)),
            };
        self.chari = closer;
        let greed = self.get_next_greedy();
        let inner = str::from_chars(
            self.chars.as_slice().slice(start + 1, closer));

        // Parse the min and max values from the regex.
        let (mut min, mut max): (uint, Option<uint>);
        if !inner.contains(",") {
            min = try!(self.parse_uint(inner));
            max = Some(min);
        } else {
            let pieces: Vec<&str> = inner.splitn(',', 1).collect();
            let (smin, smax) = (*pieces.get(0), *pieces.get(1));
            if smin.len() == 0 {
                return self.synerr("Max repetitions cannot be specified \
                                    without min repetitions.")
            }
            min = try!(self.parse_uint(smin));
            max =
                if smax.len() == 0 {
                    None
                } else {
                    Some(try!(self.parse_uint(smax)))
                };
        }

        // Do some bounds checking and make sure max >= min.
        if min > MAX_REPEAT {
            return self.synerr(format!(
                "{} exceeds maximum allowed repetitions ({})",
                min, MAX_REPEAT));
        }
        if max.is_some() {
            let m = max.unwrap();
            if m > MAX_REPEAT {
                return self.synerr(format!(
                    "{} exceeds maximum allowed repetitions ({})",
                    m, MAX_REPEAT));
            }
            if m < min {
                return self.synerr(format!(
                    "Max repetitions ({}) cannot be smaller than min \
                     repetitions ({}).", m, min));
            }
        }

        // Now manipulate the AST be repeating elements.
        if min > 0 && max.is_none() {
            // Require N copies of what's on the stack and then repeat it.
            let ast = try!(self.pop_ast());
            for _ in iter::range(0, min) {
                self.push(ast.clone())
            }
            self.push(~Rep(ast, ZeroMore, greed));
        } else {
            // Require N copies of what's on the stack and then repeat it
            // up to M times optionally.
            let ast = try!(self.pop_ast());
            for _ in iter::range(0, min) {
                self.push(ast.clone())
            }
            if max.is_some() {
                for _ in iter::range(min, max.unwrap()) {
                    self.push(~Rep(ast.clone(), ZeroOne, greed))
                }
            }
            // It's possible that we popped something off the stack but
            // never put anything back on it. To keep things simple, add
            // a no-op expression.
            if min == 0 && (max.is_none() || max == Some(0)) {
                self.push(~Nothing)
            }
        }
        Ok(())
    }

    // Parses all escape sequences.
    // Assumes that '\' has already been consumed.
    fn parse_escape(&mut self) -> Result<~Ast, Error> {
        self.next_char();
        let c = self.cur();
        if is_punct(c) {
            return Ok(~Literal(c, false))
        }
        match c {
            'a' => Ok(~Literal('\x07', false)),
            'f' => Ok(~Literal('\x0C', false)),
            't' => Ok(~Literal('\t', false)),
            'n' => Ok(~Literal('\n', false)),
            'r' => Ok(~Literal('\r', false)),
            'v' => Ok(~Literal('\x0B', false)),
            'A' => Ok(~Begin(false)),
            'z' => Ok(~End(false)),
            'b' => Ok(~WordBoundary(true)),
            'B' => Ok(~WordBoundary(false)),
            '0'|'1'|'2'|'3'|'4'|'5'|'6'|'7' => Ok(try!(self.parse_octal())),
            'x' => Ok(try!(self.parse_hex())),
            'p' | 'P' => Ok(try!(self.parse_unicode_name())),
            'd' | 'D' | 's' | 'S' | 'w' | 'W' => {
                let name = str::from_char(c.to_lowercase());
                match find_class(PERL_CLASSES, name) {
                    None => return self.err(Bug, format!(
                        "Could not find Perl class '{}'", c)),
                    Some(ranges) => {
                        let negated = c.is_uppercase();
                        let casei = self.flags.is_set(CaseI);
                        Ok(~Class(combine_ranges(ranges), negated, casei))
                    }
                }
            }
            _ => self.synerr(format!("Invalid escape sequence '\\\\{}'", c)),
        }
    }

    // Parses a unicode character class name, either of the form \pF where
    // F is a one letter unicode class name or of the form \p{name} where
    // name is the unicode class name.
    // Assumes that \p or \P has been read.
    fn parse_unicode_name(&mut self) -> Result<~Ast, Error> {
        let negated = self.cur() == 'P';
        let mut name: ~str;
        if self.peek_is(1, '{') {
            self.next_char();
            let closer =
                match self.pos('}') {
                    Some(i) => i,
                    None => return self.synerr(format!(
                        "Missing '\\}' for unclosed '\\{' at position {}",
                        self.chari)),
                };
            if closer - self.chari + 1 == 0 {
                return self.synerr("No Unicode class name found.")
            }
            name = self.slice(self.chari + 1, closer);
            self.chari = closer;
        } else {
            if self.chari + 1 >= self.chars.len() {
                return self.synerr("No single letter Unicode class name found.")
            }
            name = self.slice(self.chari + 1, self.chari + 2);
            self.chari += 1;
        }
        match find_class(UNICODE_CLASSES, name) {
            None => return self.synerr(format!(
                "Could not find Unicode class '{}'", name)),
            Some(ranges) => {
                let casei = self.flags.is_set(CaseI);
                Ok(~Class(ranges, negated, casei))
            }
        }
    }

    // Parses an octal number, up to 3 digits.
    // Assumes that \n has been read, where n is the first digit.
    fn parse_octal(&mut self) -> Result<~Ast, Error> {
        let start = self.chari;
        let mut end = start + 1;
        let (d2, d3) = (self.peek(1), self.peek(2));
        if d2 >= Some('0') && d2 <= Some('7') {
            self.next_char();
            end += 1;
            if d3 >= Some('0') && d3 <= Some('7') {
                self.next_char();
                end += 1;
            }
        }
        let s = self.slice(start, end);
        match num::from_str_radix::<u32>(s, 8) {
            Some(n) => Ok(~Literal(try!(self.char_from_u32(n)), false)),
            None => self.synerr(format!(
                "Could not parse '{}' as octal number.", s)),
        }
    }

    // Parse a hex number. Either exactly two digits or anything in {}.
    // Assumes that \x has been read.
    fn parse_hex(&mut self) -> Result<~Ast, Error> {
        if !self.peek_is(1, '{') {
            self.next_char();
            return self.parse_hex_two()
        }
        let start = self.chari + 2;
        let closer =
            match self.pos('}') {
                None => return self.synerr(format!(
                    "Missing '\\}' for unclosed '\\{' at position {}", start)),
                Some(i) => i,
            };
        self.chari = closer;
        self.parse_hex_digits(self.slice(start, closer))
    }

    // Parses a two-digit hex number.
    // Assumes that \xn has been read, where n is the first digit.
    fn parse_hex_two(&mut self) -> Result<~Ast, Error> {
        let (start, end) = (self.chari, self.chari + 2);
        if end > self.chars.len() {
            return self.synerr(format!(
                "Invalid hex escape sequence '{}'",
                self.slice(self.chari - 2, self.chars.len())))
        }
        self.next_char();
        self.parse_hex_digits(self.slice(start, end))
    }

    fn parse_hex_digits(&self, s: &str) -> Result<~Ast, Error> {
        match num::from_str_radix::<u32>(s, 16) {
            Some(n) => Ok(~Literal(try!(self.char_from_u32(n)), false)),
            None => self.synerr(format!(
                "Could not parse '{}' as hex number.", s)),
        }
    }

    // Parses a named capture.
    // Assumes that '(?' has been consumed and that the next two characters
    // are 'P' and '<'.
    fn parse_named_capture(&mut self) -> Result<(), Error> {
        self.chari += 2;
        let closer =
            match self.pos('>') {
                Some(i) => i,
                None => return self.synerr("Capture name must end with '>'."),
            };
        if closer - self.chari == 0 {
            return self.synerr("Capture names must have at least 1 character.")
        }
        let name = self.slice(self.chari, closer);
        if !name.chars().all(is_valid_cap) {
            return self.synerr(
                "Capture names must can only have underscores, \
                 letters and digits.")
        }
        self.chari = closer;
        self.caps += 1;
        self.stack.push(Paren(self.flags, self.caps, name));
        Ok(())
    }

    // Parses non-capture groups and options.
    // Assumes that '(?' has already been consumed.
    fn parse_group_opts(&mut self) -> Result<(), Error> {
        if self.cur() == 'P' && self.peek_is(1, '<') {
            return self.parse_named_capture()
        }
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
                        self.stack.push(Paren(self.flags, 0, ~""));
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

    fn get_next_greedy(&mut self) -> Greed {
        if self.peek_is(1, '?') {
            self.next_char();
            Ungreedy
        } else {
            Greedy
        }.swap(self.flags.is_set(SwapGreed))
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

    fn parse_uint(&self, s: &str) -> Result<uint, Error> {
        match from_str::<uint>(s) {
            Some(i) => Ok(i),
            None => self.synerr(format!(
                "Expected an unsigned integer but got '{}'.", s)),
        }
    }

    fn char_from_u32(&self, n: u32) -> Result<char, Error> {
        match char::from_u32(n) {
            Some(c) => Ok(c),
            None => self.synerr(format!(
                "Could not decode '{}' to unicode character.", n)),
        }
    }

    fn pos(&self, c: char) -> Option<uint> {
        self.chars.iter()
            .skip(self.chari).position(|&c2| c2 == c).map(|i| self.chari + i)
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

pub fn is_punct(c: char) -> bool {
    match c {
        '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' |
        '[' | ']' | '{' | '}' | '^' | '$' => true,
        _ => false,
    }
}

fn is_valid_cap(c: char) -> bool {
    c == '_' || (c >= '0' && c <= '9')
    || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
}

fn find_class(classes: Class, name: &str) -> Option<Vec<(char, char)>> {
    match classes.bsearch(|&(s, _)| s.cmp(&name)) {
        Some(i) => Some(Vec::from_slice(classes[i].val1())),
        None => None,
    }
}

type Class = &'static [(&'static str, &'static [(char, char)])];

static ASCII_CLASSES: Class = &[
    // Classes must be in alphabetical order so that bsearch works.
    // [:alnum:]      alphanumeric (== [0-9A-Za-z]) 
    // [:alpha:]      alphabetic (== [A-Za-z]) 
    // [:ascii:]      ASCII (== [\x00-\x7F]) 
    // [:blank:]      blank (== [\t ]) 
    // [:cntrl:]      control (== [\x00-\x1F\x7F]) 
    // [:digit:]      digits (== [0-9]) 
    // [:graph:]      graphical (== [!-~])
    // [:lower:]      lower case (== [a-z]) 
    // [:print:]      printable (== [ -~] == [ [:graph:]]) 
    // [:punct:]      punctuation (== [!-/:-@[-`{-~]) 
    // [:space:]      whitespace (== [\t\n\v\f\r ]) 
    // [:upper:]      upper case (== [A-Z]) 
    // [:word:]       word characters (== [0-9A-Za-z_]) 
    // [:xdigit:]     hex digit (== [0-9A-Fa-f]) 
    // Taken from: http://golang.org/pkg/regexp/syntax/
    ("alnum", &[('0', '9'), ('A', 'Z'), ('a', 'z')]),
    ("alpha", &[('A', 'Z'), ('a', 'z')]),
    ("ascii", &[('\x00', '\x7F')]),
    ("blank", &[(' ', ' '), ('\t', '\t')]),
    ("cntrl", &[('\x00', '\x1F'), ('\x7F', '\x7F')]),
    ("digit", &[('0', '9')]),
    ("graph", &[('!', '~')]),
    ("lower", &[('a', 'z')]),
    ("print", &[(' ', '~')]),
    ("punct", &[('!', '/'), (':', '@'), ('[', '`'), ('{', '~')]),
    ("space", &[('\t', '\t'), ('\n', '\n'), ('\x0B', '\x0B'), ('\x0C', '\x0C'),
                ('\r', '\r'), (' ', ' ')]),
    ("upper", &[('A', 'Z')]),
    ("word", &[('0', '9'), ('A', 'Z'), ('a', 'z'), ('_', '_')]),
    ("xdigit", &[('0', '9'), ('A', 'F'), ('a', 'f')]),
];

static PERL_CLASSES: Class = &[
    // Classes must be in alphabetical order so that bsearch works.
    // \d             digits (== [0-9]) 
    // \D             not digits (== [^0-9]) 
    // \s             whitespace (== [\t\n\f\r ]) 
    // \S             not whitespace (== [^\t\n\f\r ]) 
    // \w             ASCII word characters (== [0-9A-Za-z_]) 
    // \W             not ASCII word characters (== [^0-9A-Za-z_]) 
    // Taken from: http://golang.org/pkg/regexp/syntax/
    //
    // The negated classes are handled in the parser.
    ("d", &[('0', '9')]),
    ("s", &[('\t', '\t'), ('\n', '\n'), ('\x0C', '\x0C'),
            ('\r', '\r'), (' ', ' ')]),
    ("w", &[('0', '9'), ('A', 'Z'), ('a', 'z'), ('_', '_')]),
];


#[cfg(test)]
mod test {
    #[test]
    #[ignore]
    fn simple() {
        debug!("{}", super::parse("a(?i)nd"));
    }
}
