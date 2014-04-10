#![crate_id = "regexp#0.1.0"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]
#![license = "UNLICENSE"]
#![doc(html_root_url = "http://burntsushi.net/rustdoc/regexp")]

//! Regular expressions for Rust.

#![feature(macro_rules, phase)]

extern crate collections;
#[phase(syntax, link)]
extern crate log;
extern crate rand;

#[cfg(bench)]
extern crate stdtest = "test";

use std::fmt;
use std::str;
use parse::is_punct;

pub use parse::{Error, ErrorKind};
pub use regexp::{Regexp, Captures, SubCaptures, SubCapturesPos};
pub use regexp::{FindCaptures, FindMatches};
pub use regexp::{Replacer, NoExpand, RegexpSplits, RegexpSplitsN};

mod compile;
mod parse;
mod regexp;
mod vm;

#[cfg(test)]
mod test;

impl fmt::Show for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f.buf, "{} error near position {}: {}",
            self.kind, self.pos, self.msg)
    }
}

/// Escapes all regular expression meta characters in `text` so that it may be
/// safely used in a regular expression as a literal string.
pub fn quote(text: &str) -> ~str {
    let mut quoted = str::with_capacity(text.len());
    for c in text.chars() {
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

/// The `program` module exists to support the `re!` macro. Do not use.
#[doc(hidden)]
pub mod program {
    pub use super::compile::{Program, MaybeStatic};
    pub use super::compile::{Inst, Char_, CharClass, Any_, Save, Jump, Split};
    pub use super::compile::{Match, EmptyBegin, EmptyEnd, EmptyWordBoundary};
    pub use super::compile::{Dynamic, Static};
}
