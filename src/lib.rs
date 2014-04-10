#![crate_id = "regexp#0.1.0"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]
#![license = "UNLICENSE"]
#![doc(html_root_url = "http://burntsushi.net/rustdoc/regexp")]

//! Regular expressions for Rust.

#![feature(macro_rules, phase, default_type_params)]

extern crate collections;
#[phase(syntax, link)]
extern crate log;
extern crate rand;

#[cfg(bench)]
extern crate stdtest = "test";

#[phase(syntax)]
#[cfg(test)]
extern crate regexp_re;

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

#[cfg(test)]
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

pub type RegexpStatic = Regexp<&'static program::StaticProgram>;

/// The `program` module exists to support the `re!` macro. Do not use.
pub mod program {
    use std::str::MaybeOwned;
    pub use super::compile::{Program, DynamicProgram, StaticProgram};
    pub use super::compile::{Inst, Char_, CharClass, Any_, Save, Jump, Split};
    pub use super::compile::{Match, EmptyBegin, EmptyEnd, EmptyWordBoundary};
    pub use super::compile::{MaybeStaticClass, DynamicClass, StaticClass};
    use super::Regexp;

    /// For the `re!` extension. Do not use.
    pub fn make_regexp(orig: &str, insts: Vec<Inst>,
                       names: Vec<Option<MaybeOwned<'static>>>,
                       prefix: Vec<char>) -> Regexp {
        Regexp {
            p: DynamicProgram {
                regex: orig.to_owned(),
                insts: insts,
                names: names,
                prefix: prefix,
            },
        }
    }
}
