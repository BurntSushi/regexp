// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_id = "regexp#0.11-pre"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]
#![license = "MIT/ASL2"]
#![doc(html_logo_url = "http://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "http://www.rust-lang.org/favicon.ico",
       html_root_url = "http://static.rust-lang.org/doc/master")]

//! This crate provides a native implementation of regular expressions that is
//! heavily based on RE2 both in syntax and in implementation. Notably,
//! backreferences and arbitrary lookahead/lookbehind assertions are not
//! provided. In return, regular expression searching provided by this package
//! has excellent worst case performance. The specific syntax supported is 
//! documented further down.
//!
//! This crate's documentation provides some simple examples, describes Unicode 
//! support and exhaustively lists the supported syntax. For more specific 
//! details on the API, please see the documentation for the `Regexp` type.
//!
//! # First example: find a date
//!
//! General use of regular expressions in this package involves compiling an 
//! expression and then using it to search, split or replace text. For example, 
//! to confirm that some text resembles a date:
//!
//! ```rust
//! use regexp::Regexp;
//! let re = match Regexp::new(r"^\d{4}-\d{2}-\d{2}$") {
//!     Ok(re) => re,
//!     Err(err) => fail!("{}", err),
//! };
//! assert_eq!(re.is_match("2014-01-01"), true);
//! ```
//!
//! Notice the use of the `^` and `$` anchors. In this crate, every expression 
//! is executed with an implicit `.*?` at the beginning and end, which allows 
//! it to match anywhere in the text. Anchors can be used to ensure that the 
//! full text matches an expression.
//!
//! This example also demonstrates the utility of raw strings in Rust, which 
//! are just like regular strings except they are prefixed with an `r` and do 
//! not process any escape sequences. For example, `"\\d"` is the same 
//! expression as `r"\d"`.
//!
//! # The `regexp!` macro
//!
//! Rust's compile time meta-programming facilities provide a way to write an 
//! `regexp!` macro which compiles regular expressions *when your program 
//! compiles*. Said differently, if you only use `regexp!` to build regular 
//! expressions in your program, then your program cannot compile with an 
//! invalid regular expression.
//!
//! To use the `regexp!` macro, you must enable the `phase` feature and import 
//! the `regexp_macros` crate as a syntax extension:
//!
//! ```rust
//! #![feature(phase)]
//! #[phase(syntax)]
//! extern crate regexp_macros;
//! extern crate regexp;
//!
//! fn main() {
//!     let re = regexp!(r"^\d{4}-\d{2}-\d{2}$");
//!     assert_eq!(re.is_match("2014-01-01"), true);
//! }
//! ```
//!
//! There are a few things worth mentioning about using the `regexp!` macro. 
//! Firstly, it compiles an expression to *static* data, which means it can 
//! live in the module scope in addition to function scope (as shown in the 
//! previous example).
//! Secondly, the `regexp!` macro *only* accepts string *literals*.
//! Thirdly, the `regexp` crate *must* be linked with the name `regexp` since 
//! the generated code depends on finding symbols in the `regexp` crate.
//!
//! In general, one should use the `regexp!` macro whenever possible since it 
//! eliminates an entire class of bugs and incurs no runtime cost for 
//! compilation of a regular expression. If your regular expression isn't known 
//! until runtime, then you can use `Regexp::new`.
//!
//! Finally, note that an expression of the form 
//! `regexp!("...").is_match("...")` is not allowed since `regexp!` produces 
//! static data that must live for the lifetime of the program. You must always 
//! bind the result of `regexp!` to some named `static` variable.
//!
//! # Example: iterating over capture groups
//!
//! This crate provides convenient iterators for matching an expression 
//! repeatedly against a search string to find successive non-overlapping 
//! matches. For example, to find all dates in a string and be able to access 
//! them by their component pieces:
//!
//! ```rust
//! # #![feature(phase)]
//! # extern crate regexp; #[phase(syntax)] extern crate regexp_macros;
//! # fn main() {
//! let re = regexp!(r"(\d{4})-(\d{2})-(\d{2})");
//! let text = "2012-03-14, 2013-01-01 and 2014-07-05";
//! for cap in re.captures_iter(text) {
//!     println!("Month: {} Day: {} Year: {}", cap.at(2), cap.at(3), cap.at(1));
//! }
//! // Output:
//! // Month: 03 Day: 14 Year: 2012
//! // Month: 01 Day: 01 Year: 2013
//! // Month: 07 Day: 05 Year: 2014
//! # }
//! ```
//!
//! Notice that the year is in the capture group indexed at `1`. This is 
//! because the *entire match* is stored in the capture group at index `0`.
//!
//! # Example: replacement with named capture groups
//!
//! Building on the previous example, perhaps we'd like to rearrange the date 
//! formats. This can be done with text replacement. But to make the code 
//! clearer, we can *name*  our capture groups and use those names as variables 
//! in our replacement text:
//!
//! ```rust
//! # #![feature(phase)]
//! # extern crate regexp; #[phase(syntax)] extern crate regexp_macros;
//! # fn main() {
//! let re = regexp!(r"(?P<y>\d{4})-(?P<m>\d{2})-(?P<d>\d{2})");
//! let before = "2012-03-14, 2013-01-01 and 2014-07-05";
//! let after = re.replace_all(before, "$m/$d/$y");
//! assert_eq!(after, ~"03/14/2012, 01/01/2013 and 07/05/2014");
//! # }
//! ```
//!
//! The `replace` methods are actually polymorphic in the replacement, which 
//! provides more flexibility than is seen here. (See the documentation for 
//! `Regexp::replace` for more details.)
//!
//! # Pay for what you use
//!
//! With respect to searching text with a regular expression, there are three 
//! questions that can be asked:
//!
//! 1. Does the text match this expression?
//! 2. If so, where does it match?
//! 3. Where are the submatches?
//!
//! Generally speaking, this crate could provide a function to answer only #3, 
//! which would subsume #1 and #2 automatically. However, it can be 
//! significantly more expensive to compute the location of submatches, so it's 
//! best not to do it if you don't need to.
//!
//! Therefore, only use what you need. For example, don't use `find` if you 
//! only need to test if an expression matches a string. (Use `is_match` 
//! instead.)
//!
//! # Unicode
//!
//! This implementation executes regular expressions **only** on sequences of 
//! UTF8 codepoints while exposing match locations as byte indices.
//!
//! Currently, only naive case folding is supported. Namely, when matching 
//! case insensitively, the characters are first converted to their uppercase 
//! forms and then compared.
//!
//! Regular expressions themselves are also **only** interpreted as a sequence 
//! of UTF8 codepoints. This means you can embed Unicode characters directly 
//! into your expression:
//!
//! ```rust
//! # #![feature(phase)]
//! # extern crate regexp; #[phase(syntax)] extern crate regexp_macros;
//! # fn main() {
//! let re = regexp!(r"(?i)Δ+");
//! assert_eq!(re.find("ΔδΔ"), Some((0, 6)));
//! # }
//! ```
//!
//! Finally, Unicode general categories and scripts are available as character 
//! classes. For example, you can match a sequence of numerals, Greek or 
//! Cherokee letters:
//!
//! ```rust
//! # #![feature(phase)]
//! # extern crate regexp; #[phase(syntax)] extern crate regexp_macros;
//! # fn main() {
//! let re = regexp!(r"[\pN\p{Greek}\p{Cherokee}]+");
//! assert_eq!(re.find("abcΔᎠβⅠᏴγδⅡxyz"), Some((3, 23)));
//! # }
//! ```
//!
//! # Syntax
//!
//! The syntax supported in this crate is almost in an exact correspondence 
//! with the syntax supported by RE2.
//!
//! ## Matching one character
//!
//! <pre class="rust">
//! .           any character except new line (includes new line with s flag)
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
//! \A    only the beginning of text (even with multi-line mode enabled)
//! \z    only the end of text (even with multi-line mode enabled)
//! \b    an ASCII word boundary (\w on one side and \W, \A, or \z on other)
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
//! # extern crate regexp; #[phase(syntax)] extern crate regexp_macros;
//! # fn main() {
//! let re = regexp!(r"(?i)a+(?-i)b+");
//! let cap = re.captures("AaAaAbbBBBb").unwrap();
//! assert_eq!(cap.at(0), "AaAaAbb");
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
//!
//! # Untrusted input
//!
//! There are two factors to consider here: untrusted regular expressions and 
//! untrusted search text.
//!
//! Currently, there are no counter-measures in place to prevent a malicious 
//! user from writing an expression that may use a lot of resources. One such 
//! example is to repeat counted repetitions: `((a{100}){100}){100}` will try 
//! to repeat the `a` instruction `100^3` times. Essentially, this means it's 
//! very easy for an attacker to exhaust your system's memory if they are 
//! allowed to execute arbitrary regular expressions. A possible solution to 
//! this is to impose a hard limit on the size of a compiled expression, but it 
//! does not yet exist.
//!
//! The story is a bit better with untrusted search text, since this crate's 
//! implementation provides `O(nm)` search where `n` is the number of 
//! characters in the search text and `m` is the number of instructions in a 
//! compiled expression.

#![feature(macro_rules, phase)]

extern crate collections;
#[cfg(test)]
extern crate stdtest = "test";
#[cfg(test)]
extern crate rand;

#[phase(syntax, link)] extern crate log;

// During tests, this links with the `regexp` crate so that the `regexp!` macro
// can be tested.
#[cfg(test, not(stage1))]
extern crate regexp;

pub use parse::Error;
pub use re::{Regexp, Captures, SubCaptures, SubCapturesPos};
pub use re::{FindCaptures, FindMatches};
pub use re::{Replacer, NoExpand, RegexpSplits, RegexpSplitsN};
pub use re::{quote, is_match, regexp};
pub use vm::{MatchKind, Exists, Location, Submatches};

mod compile;
mod parse;
mod re;
mod vm;

#[cfg(test)]
mod test;

/// The `program` module exists to support the `regexp!` macro. Do not use.
#[doc(hidden)]
pub mod program {
    // Exporting this stuff is bad form, but it's necessary for two reasons.
    // Firstly, the `regexp!` syntax extension is in a different crate and 
    // requires access to the representation of a regexp (particularly the 
    // instruction set) in order to generate an AST. This could be mitigated if 
    // `regexp!` was defined in the same crate, but this has undesirable 
    // consequences (such as requiring a dependency on `libsyntax`).
    //
    // Secondly, the AST generated by `regexp!` must *also* be able to access 
    // the representation of a regexp program so that it may be constructed
    // staticly. Yes, this means a user program will actually construct a
    // regexp program using actual instructions, but it's all hidden behind the 
    // `regexp!` macro. This, AFAIK, is impossible to mitigate.
    //
    // For similar reasons, the representation of `Regexp` is also exported
    // but is hidden in the public API documentation.
    //
    // On the bright side, `rustdoc` lets us hide this from the public API
    // documentation, which is an acceptable compromise IMO.
    pub use super::parse::Flags;
    pub use super::re::{Dynamic, Native};
    pub use super::compile::{
        Program,
        Inst, OneChar, CharClass, Any, Save, Jump, Split,
        Match, EmptyBegin, EmptyEnd, EmptyWordBoundary,
    };
}
