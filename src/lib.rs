#![crate_id = "regexp#0.1.0"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]
#![license = "UNLICENSE"]
#![doc(html_root_url = "http://burntsushi.net/rustdoc/regexp")]

//! This crate provides a native implementation of regular expressions that is
//! heavily based on RE2 both in syntax and in implementation. Notably,
//! backreferences and arbitrary lookahead/lookbehind assertions are not
//! provided. In return, regular expression searching provided by this package
//! has excellent worst case performance. The specific syntax supported is 
//! documented further down.
//!
//! # Syntax
//!
//! The syntax supported in this crate is almost in an exact correspondence 
//! with the syntax supported by RE2.
//!
//! ## Matching one character
//!
//! <pre class="rust">
//! .           any character except new line (includes new line with 's' flag)
//! [xyz]       A character class matching either x, y or z.
//! [^xyz]      A character class matching any character except x, y and z.
//! [a-z]       A character class matching any character in range a-z.
//! \d          Perl character class ([0-9])
//! \D          Negated Perl character class ([^0-9])
//! [:alpha:]   ASCII character class ([A-Za-z])
//! [:^alpha:]  Negated ASCII character class ([^A-Za-z])
//! \pN         One letter name Unicode character class
//! \p{Greek}   Unicode character class (general category or script)
//! \PN         Negated one letter name Unicode character class
//! \P{Greek}   negated Unicode character class (general category or script)
//! </pre>
//!
//! Any named character class may appear inside a bracketed `[...]` character
//! class. For example, `[\p{Greek}\pN]` matches any Greek or numeral 
//! character.
//!
//! ## Composites
//!
//! <pre class="rust">
//! xy    concatenation (x followed by y)
//! x|y   alternation (x or y, prefer x)
//! </pre>
//!
//! ## Repetitions
//!
//! <pre class="rust">
//! x*        zero or more of x (greedy)
//! x+        one or more of x (greedy)
//! x?        zero or one of x (greedy)
//! x*?       zero or more of x (ungreedy)
//! x+?       one or more of x (ungreedy)
//! x??       zero or one of x (ungreedy)
//! x{n,m}    at least n and at most x (greedy)
//! x{n,}     at least n x (greedy)
//! x{n}      exactly n x
//! x{n,m}?   at least n and at most x (ungreedy)
//! x{n,}?    at least n x (ungreedy)
//! x{n}?     exactly n x
//! </pre>
//!
//! ## Empty matches
//!
//! <pre class="rust">
//! ^     the beginning of text
//! $     the end of text
//! \A    the beginning of text (even with multi-line mode enabled)
//! \z    the end of text (even with multi-line mode enabled)
//! \b    an ASCII word boundary (\w on one size and \W, \A, or \z on other)
//! \B    not an ASCII word boundary
//! </pre>
//!
//! ## Grouping and flags
//!
//! <pre class="rust">
//! (exp)          numbered capture group (indexed by opening parenthesis)
//! (?P&lt;name&gt;exp)  named capture group (also numbered)
//! (?:exp)        non-capturing group
//! (?flags)       set flags within current group
//! (?flags:exp)   set flags for exp (non-capturing)
//! </pre>
//!
//! Flags are each a single character. For example, `(?x)` sets the flag `x`
//! and `(?-x)` clears the flag `x`. Multiple flags can be set or cleared at
//! the same time: `(?xy)` sets both the `x` and `y` flags and `(?x-y)` sets 
//! the `x` flag and clears the `y` flag.
//!
//! All flags are by default disabled. They are:
//!
//! <pre class="rust">
//! i     case insensitive
//! m     multi-line mode: ^ and $ match begin/end of line
//! s     allow . to match \n
//! U     swap the meaning of `x*` and `x*?`
//! </pre>
//!
//! Here's an example that matches case insensitively for only part of the 
//! expression:
//!
//! ```rust
//! # #![feature(phase)]
//! # extern crate regexp; #[phase(syntax)] extern crate regexp_re;
//! # use regexp::Regexp; fn main() {
//! static re: Regexp = re!(r"(?i)a+(?-i)b+");
//! assert_eq!(re.find("AaAaAbbBBBb"), Some((0, 7)));
//! # }
//! ```
//!
//! Notice that the `a+` matches either `a` or `A`, but the `b+` only matches
//! `b`.
//!
//! ## Escape sequences
//!
//! <pre class="rust">
//! \*         literal *, works for any punctuation character: \.+*?()|[]{}^$
//! \a         bell (\x07)
//! \f         form feed (\x0C)
//! \t         horizontal tab
//! \n         new line
//! \r         carriage return
//! \v         vertical tab (\x0B)
//! \123       octal character code (up to three digits)
//! \x7F       hex character code (exactly two digits)
//! \x{10FFFF} any hex character code corresponding to a valid UTF8 codepoint
//! </pre>
//!
//! ## Perl character classes
//!
//! <pre class="rust">
//! \d     digit ([0-9])
//! \D     not digit
//! \s     whitespace ([\t\n\f\r ])
//! \S     not whitespace
//! \w     ASCII word character ([0-9A-Za-z_])
//! \W     not ASCII word character
//! </pre>
//!
//! ## ASCII character classes
//!
//! <pre class="rust">
//! [:alnum:]    alphanumeric ([0-9A-Za-z]) 
//! [:alpha:]    alphabetic ([A-Za-z]) 
//! [:ascii:]    ASCII ([\x00-\x7F]) 
//! [:blank:]    blank ([\t ]) 
//! [:cntrl:]    control ([\x00-\x1F\x7F]) 
//! [:digit:]    digits ([0-9]) 
//! [:graph:]    graphical ([!-~])
//! [:lower:]    lower case ([a-z]) 
//! [:print:]    printable ([ -~])
//! [:punct:]    punctuation ([!-/:-@[-`{-~]) 
//! [:space:]    whitespace ([\t\n\v\f\r ]) 
//! [:upper:]    upper case ([A-Z]) 
//! [:word:]     word characters ([0-9A-Za-z_]) 
//! [:xdigit:]   hex digit ([0-9A-Fa-f]) 
//! </pre>

#![feature(macro_rules, phase)]

extern crate collections;
#[phase(syntax, link)]
extern crate log;
extern crate rand;

#[cfg(bench)]
extern crate stdtest = "test";

// During tests, this links with the `regexp` and `regexp_re` crates to provide
// the `re!` macro (so it can be tested).
// We don't do this for benchmarks since it (currently) prevents compiling
// with -Z lto.
#[cfg(test, not(bench))]
#[phase(syntax)]
extern crate regexp_re;
#[cfg(test, not(bench))]
extern crate regexp;

pub use parse::Error;
pub use re::{Regexp, Captures, SubCaptures, SubCapturesPos};
pub use re::{FindCaptures, FindMatches};
pub use re::{Replacer, NoExpand, RegexpSplits, RegexpSplitsN};
pub use re::{quote, is_match, regexp};

mod compile;
mod parse;
mod re;
mod vm;

#[cfg(test)]
mod test;

/// The `program` module exists to support the `re!` macro. Do not use.
#[doc(hidden)]
pub mod program {
    // Exporting this stuff is bad form, but it's necessary for two reasons.
    // Firstly, the `re!` syntax extension is in a different crate and requires
    // access to the representation of a regexp (particularly the instruction
    // set) in order to generate an AST. This could be mitigated if `re!` was
    // defined in the same crate, but this has undesirable consequences (such
    // as requiring a dependency on `libsyntax`).
    // Secondly, the AST generated by `re!` must *also* be able to access the
    // representation of a regexp program so that it may be constructed
    // staticly. Yes, this means a user program will actually construct a
    // regexp program using actual instructions, but it's all hidden behind the 
    // `re!` macro. This, AFAIK, is impossible to mitigate.
    //
    // For similar reasons, the representation of `Regexp` is also exported
    // but is hidden in the public API documentation.
    //
    // On the bright side, `rustdoc` lets us hide this from the public API
    // documentation, which is an acceptable compromise IMO.
    pub use super::parse::Flags;
    pub use super::compile::{
        Program, MaybeStatic,
        Inst, OneChar, CharClass, Any, Save, Jump, Split,
        Match, EmptyBegin, EmptyEnd, EmptyWordBoundary,
        Dynamic, Static,
    };
}
