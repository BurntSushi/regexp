#![crate_id = "regexp#0.1.0"]
#![crate_type = "lib"]
#![license = "UNLICENSE"]
#![doc(html_root_url = "http://burntsushi.net/rustdoc/regexp")]

#![allow(unused_imports)]
#![allow(dead_code)]

//! Regular expressions for Rust.

#![feature(macro_rules)]
#![feature(phase)]

extern crate collections;
#[phase(syntax, link)]
extern crate log;
extern crate quickcheck;
extern crate rand;

use std::fmt;
use std::str;
use parse::is_punct;

pub use regexp::{Regexp, Captures, SubCaptures, SubCapturesPos};
pub use regexp::{FindCaptures, FindMatches};
pub use regexp::{Replacer, NoExpand, RegexpSplits, RegexpSplitsN};

mod compile;
mod parse;
mod regexp;
mod vm;
mod test;

/// Error corresponds to something that can go wrong while parsing or compiling
/// a regular expression.
///
/// (Once an expression is compiled, it is not possible to produce an error
/// via searching, splitting or replacing.)
pub struct Error {
    pub pos: uint,
    pub kind: ErrorKind,
    pub msg: ~str,
}

/// Describes the type of the error returned.
#[deriving(Show)]
pub enum ErrorKind {
    Bug,
    BadSyntax,
}

impl fmt::Show for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f.buf, "{} error near position {}: {}",
            self.kind, self.pos, self.msg)
    }
}

/// Escapes all regular expression meta characters in `s` so that it may be
/// safely used in a regular expression as a literal string.
pub fn quote(s: &str) -> ~str {
    let mut quoted = str::with_capacity(s.len());
    for c in s.chars() {
        if is_punct(c) {
            quoted.push_char('\\')
        }
        quoted.push_char(c);
    }
    quoted
}

/// Tests if the given regular expression matches somewhere in the text given.
///
/// If there was a problem compiling the regular expression, an error is
/// returned.
pub fn is_match(regex: &str, text: &str) -> Result<bool, Error> {
    Regexp::new(regex).map(|r| r.is_match(text))
}

#[cfg(test)]
mod testold {
    use super::compile;
    use super::parse;
    use super::regexp::{Regexp, NoExpand};
    use super::vm;

    #[test]
    #[ignore]
    fn other() {
        let r = Regexp::new(r"(\S+)\s+(?P<last>\S+)\s*").unwrap();
        let text = "andrew gallant    kaitlyn brady";
        debug!("Replaced: {}", r.replace_all(text, "$last,$wat $1 "));

        // let r = Regexp::new("a+").unwrap(); 
        // let text = "aaaawhoa"; 
        // for m in r.captures_iter(text) { 
            // debug!("Match: {} (pos: {})", m.at(0), m.pos(0)); 
        // } 
    }

    fn run(re: &str, text: &str) {
        let r = match Regexp::new(re) {
            Err(err) => fail!("{}", err),
            Ok(r) => r,
        };
        for (s, e) in r.find_iter(text) {
            debug!("Matched: {} ({})", (s, e), text.slice(s, e));
        }
        for cap in r.captures_iter(text) {
            debug!("Captures: {}", cap.iter().collect::<Vec<&str>>());
        }
        // let gs = r.captures(text).unwrap(); 
        // let all: Vec<&str> = gs.iter().collect(); 
        // debug!("All: {}, First: {}, Second: {}", all, gs.at(0), gs.at(1)); 
        // debug!("Named: {}", gs.name("sec")); 

    }

    #[test]
    #[ignore]
    fn simple() {
        // run("(?i:and)rew", "aNdrew"); 
        // run("a+b+?", "abbbbb"); 
        // run("(?s:.+)", "abb\nbbb"); 
        // run("(a*)+", "aaa"); 
        // run("(?sm)(.*?)^ab", "\n\n\nab"); 
        // run("(?sm)ab$\n", "ab\n"); 
        // run("(a{2}){3}", "aaaaaa"); 
        // run("a{2,}", "aaaaaa"); 
        // run("[a-z0-9]+", "ab0cdef"); 
        // run("<([^>])+>", "<strong>hello there</strong>"); 
        // run("(a|bcdef|g|ab|c|d|e|efg|fg)*", "abcdefg"); 
        // run("[^[:^alnum:]]+", "abc0123"); 

        // run(r"[\D]+", "abc123abc"); 
        // run(r".*([a-z]\b)", "andrew gallant"); 
        // run(r"\**", "**"); 
        // run(r"[\A]+", "-]a^a-a"); 
        // run(r"[^\P{N}\P{Cherokee}]+", "aᏑⅡᏡⅥ"); 
        // run(r"[^\P{N}\P{Cherokee}]+", "aᏑⅡᏡⅥ"); 
        // run("(?i)[^a-z]+", "ANDREW"); 

        // run(r"dre", "andrew dr. dre yo"); 

        // let roman = ~"ⅡⅢⅳⅥ"; 
        // run(r"\pN(?P<sec>\pN)", roman); 
        // run(r"\pN+", roman); 

        let text = "abaabaccadaaae";
        let re = Regexp::new("a*").unwrap();
        // for (s, e) in re.find_iter(text) { 
            // debug!("Find: ({}, {})", s, e); 
        // } 
        for s in re.splitn(text, 2) {
            debug!("Split: {}", s);
        }
    }

    #[test]
    // #[ignore] 
    fn captures() {
        // run("(a)b", "ab"); 
        // run("(?sm)(.*)^\nab", "\n\n\nab"); 
        // run(r"(?P<wat>\d+)a(?P<a>\d+)", "123a456"); 
    }
}
