// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use collections::HashMap;
use std::cast;
use std::from_str::from_str;
use std::str::{MaybeOwned, Owned, Slice};
use std::str::raw;

use std::mem;
use std::ptr;
use std::rt::global_heap::malloc_raw;
use RawVec = std::raw::Vec;

use compile::Program;
use parse;
use vm;
use vm::{CaptureLocs, MatchKind, Exists, Location, Submatches};

/// Escapes all regular expression meta characters in `text` so that it may be
/// safely used in a regular expression as a literal string.
pub fn quote(text: &str) -> ~str {
    let mut quoted = StrBuf::with_capacity(text.len());
    for c in text.chars() {
        if parse::is_punct(c) {
            quoted.push_char('\\')
        }
        quoted.push_char(c);
    }
    quoted.into_owned()
}

/// Tests if the given regular expression matches somewhere in the text given.
///
/// If there was a problem compiling the regular expression, an error is
/// returned.
///
/// To find submatches, split or replace text, you'll need to compile an
/// expression first.
///
/// Note that you should prefer the `regexp!` macro when possible. For example,
/// `regexp!("...").is_match("...")`.
pub fn is_match(regex: &str, text: &str) -> Result<bool, parse::Error> {
    Regexp::new(regex).map(|r| r.is_match(text))
}

/// Regexp is a compiled regular expression, represented as either a sequence
/// of bytecode instructions (dynamic) or as a specialized Rust function
/// (native). It can be used to search, split
/// or replace text. All searching is done with an implicit `.*?` at the
/// beginning and end of an expression. To force an expression to match the
/// whole string (or a prefix or a suffix), you must use an anchor like `^` or
/// `$` (or `\A` and `\z`).
///
/// While this crate will handle Unicode strings (whether in the regular
/// expression or in the search text), all positions returned are **byte
/// indices**. Every byte index is guaranteed to be at a UTF8 codepoint
/// boundary.
///
/// The lifetimes `'r` and `'t` in this crate correspond to the lifetime of a
/// compiled regular expression and text to search, respectively.
///
/// The only methods that allocate new strings are the string replacement
/// methods. All other methods (searching and splitting) return borrowed
/// pointers into the string given.
///
/// # Examples
///
/// Find the location of a US phone number:
///
/// ```rust
/// # use regexp::Regexp;
/// let re = match Regexp::new("[0-9]{3}-[0-9]{3}-[0-9]{4}") {
///     Ok(re) => re,
///     Err(err) => fail!("{}", err),
/// };
/// assert_eq!(re.find("phone: 111-222-3333"), Some((7, 19)));
/// ```
///
/// You can also use the `regexp!` macro to compile a regular expression when
/// you compile your program:
///
/// ```rust
/// #![feature(phase)]
/// extern crate regexp;
/// #[phase(syntax)] extern crate regexp_macros;
///
/// fn main() {
///     let re = regexp!(r"\d+");
///     assert_eq!(re.find("123 abc"), Some((0, 3)));
/// }
/// ```
///
/// Given an incorrect regular expression, `regexp!` will cause the Rust
/// compiler to produce a compile time error.
/// Note that `regexp!` will compile the expression to native Rust code, which
/// makes it much faster when searching text.
/// More details about the `regexp!` macro can be found in the `regexp` crate
/// documentation.
#[deriving(Clone)]
#[allow(visible_private_types)]
pub struct Regexp {
    /// The representation of `Regexp` is exported to support the `regexp!`
    /// syntax extension. Do not rely on it.
    ///
    /// See the comments for the `program` module in `lib.rs` for a more
    /// detailed explanation for what `regexp!` requires.
    #[doc(hidden)]
    pub original: ~str,
    #[doc(hidden)]
    pub names: ~[Option<~str>],
    #[doc(hidden)]
    pub p: MaybeNative,
}

pub enum MaybeNative {
    Dynamic(Program),
    Native(fn(MatchKind, &str, uint, uint) -> Vec<Option<uint>>),
}

impl Clone for MaybeNative {
    fn clone(&self) -> MaybeNative {
        match *self {
            Dynamic(ref p) => Dynamic(p.clone()),
            Native(fp) => Native(fp),
        }
    }
}

impl Regexp {
    /// Compiles a dynamic regular expression. Once compiled, it can be
    /// used repeatedly to search, split or replace text in a string.
    ///
    /// When possible, you should prefer the `regexp!` macro since it is
    /// safer and always faster.
    ///
    /// If an invalid expression is given, then an error is returned.
    pub fn new(re: &str) -> Result<Regexp, parse::Error> {
        let ast = try!(parse::parse(re));
        let (prog, names) = Program::new(ast);
        Ok(Regexp { original: re.to_owned(), names: names, p: Dynamic(prog) })
    }
}

impl Regexp {
    /// Returns true if and only if the regexp matches the string given.
    pub fn is_match(&self, text: &str) -> bool {
        has_match(&exec(self, Exists, text))
    }

    /// Returns the start and end byte range of the leftmost-first match in
    /// `text`. If no match exists, then `None` is returned.
    ///
    /// Note that this should only be used if you want to discover the position
    /// of the match. Testing the existence of a match is faster if you use
    /// `is_match`.
    pub fn find(&self, text: &str) -> Option<(uint, uint)> {
        let caps = exec(self, Location, text);
        if has_match(&caps) {
            Some((caps.get(0).unwrap(), caps.get(1).unwrap()))
        } else {
            None
        }
    }

    /// Returns an iterator for each successive non-overlapping match in
    /// `text`, returning the start and end byte indices with respect to
    /// `text`.
    pub fn find_iter<'r, 't>(&'r self, text: &'t str) -> FindMatches<'r, 't> {
        FindMatches {
            re: self,
            search: text,
            last_end: 0,
            last_match: None,
        }
    }

    /// Returns the capture groups corresponding to the leftmost-first
    /// match in `text`. Capture group `0` always corresponds to the entire
    /// match. If no match is found, then `None` is returned.
    ///
    /// You should only use `captures` if you need access to submatches.
    /// Otherwise, `find` is faster for discovering the location of the overall
    /// match.
    pub fn captures<'t>(&self, text: &'t str) -> Option<Captures<'t>> {
        let caps = exec(self, Submatches, text);
        Captures::new(self, text, caps)
    }

    /// Returns an iterator over all the non-overlapping capture groups matched
    /// in `text`. This is operationally the same as `find_iter` (except it
    /// yields information about submatches).
    pub fn captures_iter<'r, 't>(&'r self, text: &'t str)
                                -> FindCaptures<'r, 't> {
        FindCaptures {
            re: self,
            search: text,
            last_match: None,
            last_end: 0,
        }
    }

    /// Returns an iterator of substrings of `text` delimited by a match
    /// of the regular expression.
    /// Namely, each element of the iterator corresponds to text that *isn't*
    /// matched by the regular expression.
    ///
    /// This method will *not* copy the text given.
    ///
    /// # Example
    ///
    /// To split a string delimited by arbitrary amounts of spaces or tabs:
    ///
    /// ```rust
    /// # #![feature(phase)]
    /// # extern crate regexp; #[phase(syntax)] extern crate regexp_macros;
    /// # fn main() {
    /// let re = regexp!(r"[ \t]+");
    /// let fields: Vec<&str> = re.split("a b \t  c\td    e").collect();
    /// assert_eq!(fields, vec!("a", "b", "c", "d", "e"));
    /// # }
    /// ```
    pub fn split<'r, 't>(&'r self, text: &'t str) -> RegexpSplits<'r, 't> {
        RegexpSplits {
            finder: self.find_iter(text),
            last: 0,
        }
    }

    /// Returns an iterator of at most `limit` substrings of `text` delimited
    /// by a match of the regular expression. (A `limit` of `0` will return no
    /// substrings.)
    /// Namely, each element of the iterator corresponds to text that *isn't*
    /// matched by the regular expression.
    /// The remainder of the string that is not split will be the last element
    /// in the iterator.
    ///
    /// This method will *not* copy the text given.
    ///
    /// # Example
    ///
    /// Get the first two words in some text:
    ///
    /// ```rust
    /// # #![feature(phase)]
    /// # extern crate regexp; #[phase(syntax)] extern crate regexp_macros;
    /// # fn main() {
    /// let re = regexp!(r"\W+");
    /// let fields: Vec<&str> = re.splitn("Hey! How are you?", 3).collect();
    /// assert_eq!(fields, vec!("Hey", "How", "are you?"));
    /// # }
    /// ```
    pub fn splitn<'r, 't>(&'r self, text: &'t str, limit: uint)
                         -> RegexpSplitsN<'r, 't> {
        RegexpSplitsN {
            splits: self.split(text),
            cur: 0,
            limit: limit,
        }
    }

    /// Replaces the leftmost-first match with the replacement provided.
    /// The replacement can be a regular string (where `$N` and `$name` are
    /// expanded to match capture groups) or a function that takes the matches'
    /// `Captures` and returns the replaced string.
    ///
    /// If no match is found, then a copy of the string is returned unchanged.
    ///
    /// # Examples
    ///
    /// Note that this function is polymorphic with respect to the replacement.
    /// In typical usage, this can just be a normal string:
    ///
    /// ```rust
    /// # #![feature(phase)]
    /// # extern crate regexp; #[phase(syntax)] extern crate regexp_macros;
    /// # fn main() {
    /// let re = regexp!("[^01]+");
    /// assert_eq!(re.replace("1078910", ""), ~"1010");
    /// # }
    /// ```
    ///
    /// But anything satisfying the `Replacer` trait will work. For example,
    /// a closure of type `|&Captures| -> ~str` provides direct access to the
    /// captures corresponding to a match. This allows one to access
    /// submatches easily:
    ///
    /// ```rust
    /// # #![feature(phase)]
    /// # extern crate regexp; #[phase(syntax)] extern crate regexp_macros;
    /// # use regexp::Captures; fn main() {
    /// let re = regexp!(r"([^,\s]+),\s+(\S+)");
    /// let result = re.replace("Springsteen, Bruce", |caps: &Captures| {
    ///     format!("{} {}", caps.at(2), caps.at(1))
    /// });
    /// assert_eq!(result, ~"Bruce Springsteen");
    /// # }
    /// ```
    ///
    /// But this is a bit cumbersome to use all the time. Instead, a simple
    /// syntax is supported that expands `$name` into the corresponding capture
    /// group. Here's the last example, but using this expansion technique
    /// with named capture groups:
    ///
    /// ```rust
    /// # #![feature(phase)]
    /// # extern crate regexp; #[phase(syntax)] extern crate regexp_macros;
    /// # fn main() {
    /// let re = regexp!(r"(?P<last>[^,\s]+),\s+(?P<first>\S+)");
    /// let result = re.replace("Springsteen, Bruce", "$first $last");
    /// assert_eq!(result, ~"Bruce Springsteen");
    /// # }
    /// ```
    ///
    /// Note that using `$2` instead of `$first` or `$1` instead of `$last`
    /// would produce the same result. To write a literal `$` use `$$`.
    ///
    /// Finally, sometimes you just want to replace a literal string with no
    /// submatch expansion. This can be done by wrapping a string with
    /// `NoExpand`:
    ///
    /// ```rust
    /// # #![feature(phase)]
    /// # extern crate regexp; #[phase(syntax)] extern crate regexp_macros;
    /// # use regexp::NoExpand; fn main() {
    /// let re = regexp!(r"(?P<last>[^,\s]+),\s+(\S+)");
    /// let result = re.replace("Springsteen, Bruce", NoExpand("$2 $last"));
    /// assert_eq!(result, ~"$2 $last");
    /// # }
    /// ```
    pub fn replace<R: Replacer>(&self, text: &str, rep: R) -> ~str {
        self.replacen(text, 1, rep)
    }

    /// Replaces all non-overlapping matches in `text` with the
    /// replacement provided. This is the same as calling `replacen` with
    /// `limit` set to `0`.
    ///
    /// See the documentation for `replace` for details on how to access
    /// submatches in the replacement string.
    pub fn replace_all<R: Replacer>(&self, text: &str, rep: R) -> ~str {
        self.replacen(text, 0, rep)
    }

    /// Replaces at most `limit` non-overlapping matches in `text` with the
    /// replacement provided. If `limit` is 0, then all non-overlapping matches
    /// are replaced.
    ///
    /// See the documentation for `replace` for details on how to access
    /// submatches in the replacement string.
    pub fn replacen<R: Replacer>
                   (&self, text: &str, limit: uint, rep: R) -> ~str {
        let mut new = StrBuf::with_capacity(text.len());
        let mut last_match = 0u;
        let mut i = 0;
        for cap in self.captures_iter(text) {
            // It'd be nicer to use the 'take' iterator instead, but it seemed
            // awkward given that '0' => no limit.
            if limit > 0 && i >= limit {
                break
            }
            i += 1;

            let (s, e) = cap.pos(0).unwrap(); // captures only reports matches
            new.push_str(unsafe { raw::slice_bytes(text, last_match, s) });
            new.push_str(rep.reg_replace(&cap).as_slice());
            last_match = e;
        }
        new.push_str(unsafe { raw::slice_bytes(text, last_match, text.len()) });

        // The lengths we will go to avoid allocation.
        // This has a *dramatic* affect on the regex-dna benchmark (and indeed,
        // any code that uses 'replace' on a large corpus of text multiple
        // times). The trick is to avoid the obscene amount of allocation
        // currently done in slice::from_iter. I've been promised that DST will
        // fix this.
        //
        // The following is based on the code in slice::from_iter, but
        // shortened since we know we're dealing with bytes. The key is that
        // we already have a Vec<u8>, so there's no reason to re-collect it
        // (which is what from_iter currently does).
        let mut xs = new.into_bytes();
        let size = mem::size_of::<RawVec<()>>().checked_add(&xs.len());
        let size = size.expect("overflow in replacen()");
        unsafe {
            let ret = malloc_raw(size) as *mut RawVec<()>;
            (*ret).fill = xs.len();
            (*ret).alloc = xs.len();
            ptr::copy_nonoverlapping_memory(
                &mut (*ret).data as *mut _ as *mut u8, xs.as_ptr(), xs.len());
            xs.set_len(0);
            cast::transmute(ret)
        }
    }

    /// Returns the original string used to construct this regular expression.
    pub fn to_str<'r>(&'r self) -> &'r str {
        self.original.as_slice()
    }
}

/// NoExpand indicates literal string replacement.
///
/// It can be used with `replace` and `replace_all` to do a literal
/// string replacement without expanding `$name` to their corresponding
/// capture groups.
///
/// `'r` is the lifetime of the literal text.
pub struct NoExpand<'t>(pub &'t str);

/// Replacer describes types that can be used to replace matches in a string.
pub trait Replacer {
    /// Returns a possibly owned string that is used to replace the match
    /// corresponding the the `caps` capture group.
    ///
    /// The `'a` lifetime refers to the lifetime of a borrowed string when
    /// a new owned string isn't needed (e.g., for `NoExpand`).
    fn reg_replace<'a>(&'a self, caps: &Captures) -> MaybeOwned<'a>;
}

impl<'t> Replacer for NoExpand<'t> {
    fn reg_replace<'a>(&'a self, _: &Captures) -> MaybeOwned<'a> {
        let NoExpand(s) = *self;
        Slice(s)
    }
}

impl<'t> Replacer for &'t str {
    fn reg_replace<'a>(&'a self, caps: &Captures) -> MaybeOwned<'a> {
        Owned(caps.expand(*self))
    }
}

impl<'a> Replacer for |&Captures|: 'a -> ~str {
    fn reg_replace<'r>(&'r self, caps: &Captures) -> MaybeOwned<'r> {
        Owned((*self)(caps))
    }
}

/// Yields all substrings delimited by a regular expression match.
///
/// `'r` is the lifetime of the compiled expression and `'t` is the lifetime
/// of the string being split.
pub struct RegexpSplits<'r, 't> {
    finder: FindMatches<'r, 't>,
    last: uint,
}

impl<'r, 't> Iterator<&'t str> for RegexpSplits<'r, 't> {
    fn next(&mut self) -> Option<&'t str> {
        let text = self.finder.search;
        match self.finder.next() {
            None => {
                if self.last >= text.len() {
                    None
                } else {
                    let s = text.slice(self.last, text.len());
                    self.last = text.len();
                    Some(s)
                }
            }
            Some((s, e)) => {
                let matched = text.slice(self.last, s);
                self.last = e;
                Some(matched)
            }
        }
    }
}

/// Yields at most `N` substrings delimited by a regular expression match.
///
/// The last substring will be whatever remains after splitting.
///
/// `'r` is the lifetime of the compiled expression and `'t` is the lifetime
/// of the string being split.
pub struct RegexpSplitsN<'r, 't> {
    splits: RegexpSplits<'r, 't>,
    cur: uint,
    limit: uint,
}

impl<'r, 't> Iterator<&'t str> for RegexpSplitsN<'r, 't> {
    fn next(&mut self) -> Option<&'t str> {
        let text = self.splits.finder.search;
        if self.cur >= self.limit {
            None
        } else {
            self.cur += 1;
            if self.cur >= self.limit {
                Some(text.slice(self.splits.last, text.len()))
            } else {
                self.splits.next()
            }
        }
    }
}

/// Captures represents a group of captured strings for a single match.
///
/// The 0th capture always corresponds to the entire match. Each subsequent
/// index corresponds to the next capture group in the regex.
/// If a capture group is named, then the matched string is *also* available
/// via the `name` method. (Note that the 0th capture is always unnamed and so
/// must be accessed with the `at` method.)
///
/// Positions returned from a capture group are always byte indices.
///
/// `'t` is the lifetime of the matched text.
pub struct Captures<'t> {
    text: &'t str,
    locs: CaptureLocs,
    named: Option<HashMap<~str, uint>>,
}

impl<'t> Captures<'t> {
    fn new(re: &Regexp, search: &'t str, locs: CaptureLocs)
          -> Option<Captures<'t>> {
        if !has_match(&locs) {
            return None
        }

        let named =
            if re.names.len() == 0 {
                None
            } else {
                let mut named = HashMap::new();
                for (i, name) in re.names.iter().enumerate() {
                    match name {
                        &None => {},
                        &Some(ref name) => {
                            named.insert(name.to_owned(), i);
                        }
                    }
                }
                Some(named)
            };
        Some(Captures {
            text: search,
            locs: locs,
            named: named,
        })
    }

    /// Returns the start and end positions of the Nth capture group.
    /// Returns `None` if `i` is not a valid capture group or if the capture
    /// group did not match anything.
    /// The positions returned are *always* byte indices with respect to the
    /// original string matched.
    pub fn pos(&self, i: uint) -> Option<(uint, uint)> {
        let (s, e) = (i * 2, i * 2 + 1);
        if e >= self.locs.len() || self.locs.get(s).is_none() {
            // VM guarantees that each pair of locations are both Some or None.
            return None
        }
        Some((self.locs.get(s).unwrap(), self.locs.get(e).unwrap()))
    }

    /// Returns the matched string for the capture group `i`.
    /// If `i` isn't a valid capture group or didn't match anything, then the
    /// empty string is returned.
    pub fn at(&self, i: uint) -> &'t str {
        match self.pos(i) {
            None => "",
            Some((s, e)) => {
                self.text.slice(s, e)
            }
        }
    }

    /// Returns the matched string for the capture group named `name`.
    /// If `name` isn't a valid capture group or didn't match anything, then
    /// the empty string is returned.
    pub fn name(&self, name: &str) -> &'t str {
        match self.named {
            None => "",
            Some(ref h) => {
                match h.find(&name.to_owned()) {
                    None => "",
                    Some(i) => self.at(*i),
                }
            }
        }
    }

    /// Creates an iterator of all the capture groups in order of appearance
    /// in the regular expression.
    pub fn iter(&'t self) -> SubCaptures<'t> {
        SubCaptures { idx: 0, caps: self, }
    }

    /// Creates an iterator of all the capture group positions in order of
    /// appearance in the regular expression. Positions are byte indices
    /// in terms of the original string matched.
    pub fn iter_pos(&'t self) -> SubCapturesPos<'t> {
        SubCapturesPos { idx: 0, caps: self, }
    }

    /// Expands all instances of `$name` in `text` to the corresponding capture
    /// group `name`.
    ///
    /// `name` may be an integer corresponding to the index of the
    /// capture group (counted by order of opening parenthesis where `0` is the
    /// entire match) or it can be a name (consisting of letters, digits or
    /// underscores) corresponding to a named capture group.
    ///
    /// If `name` isn't a valid capture group (whether the name doesn't exist or
    /// isn't a valid index), then it is replaced with the empty string.
    ///
    /// To write a literal `$` use `$$`.
    pub fn expand(&self, text: &str) -> ~str {
        // How evil can you get?
        // FIXME: Don't use regexes for this. It's completely unnecessary.
        let re = Regexp::new(r"(^|[^$]|\b)\$(\w+)").unwrap();
        let text = re.replace_all(text, |refs: &Captures| -> ~str {
            let (pre, name) = (refs.at(1), refs.at(2));
            pre + match from_str::<uint>(name) {
                None => self.name(name).to_owned(),
                Some(i) => self.at(i).to_owned(),
            }
        });
        text.replace("$$", "$")
    }
}

impl<'t> Container for Captures<'t> {
    /// Returns the number of captured groups.
    #[inline]
    fn len(&self) -> uint {
        self.locs.len() / 2
    }
}

/// An iterator over capture groups for a particular match of a regular
/// expression.
///
/// `'t` is the lifetime of the matched text.
pub struct SubCaptures<'t> {
    idx: uint,
    caps: &'t Captures<'t>,
}

impl<'t> Iterator<&'t str> for SubCaptures<'t> {
    fn next(&mut self) -> Option<&'t str> {
        if self.idx < self.caps.len() {
            self.idx += 1;
            Some(self.caps.at(self.idx - 1))
        } else {
            None
        }
    }
}

/// An iterator over capture group positions for a particular match of a
/// regular expression.
///
/// Positions are byte indices in terms of the original string matched.
///
/// `'t` is the lifetime of the matched text.
pub struct SubCapturesPos<'t> {
    idx: uint,
    caps: &'t Captures<'t>,
}

impl<'t> Iterator<Option<(uint, uint)>> for SubCapturesPos<'t> {
    fn next(&mut self) -> Option<Option<(uint, uint)>> {
        if self.idx < self.caps.len() {
            self.idx += 1;
            Some(self.caps.pos(self.idx - 1))
        } else {
            None
        }
    }
}

/// An iterator that yields all non-overlapping capture groups matching a
/// particular regular expression. The iterator stops when no more matches can
/// be found.
///
/// `'r` is the lifetime of the compiled expression and `'t` is the lifetime
/// of the matched string.
pub struct FindCaptures<'r, 't> {
    re: &'r Regexp,
    search: &'t str,
    last_match: Option<uint>,
    last_end: uint,
}

impl<'r, 't> Iterator<Captures<'t>> for FindCaptures<'r, 't> {
    fn next(&mut self) -> Option<Captures<'t>> {
        if self.last_end > self.search.len() {
            return None
        }

        let caps = exec_slice(self.re, Submatches, self.search,
                              self.last_end, self.search.len());
        let (s, e) =
            if !has_match(&caps) {
                return None
            } else {
                (caps.get(0).unwrap(), caps.get(1).unwrap())
            };

        // Don't accept empty matches immediately following a match.
        // i.e., no infinite loops please.
        if e - s == 0 && Some(self.last_end) == self.last_match {
            self.last_end += 1;
            return self.next()
        }
        self.last_end = e;
        self.last_match = Some(self.last_end);
        Captures::new(self.re, self.search, caps)
    }
}

/// An iterator over all non-overlapping matches for a particular string.
///
/// The iterator yields a tuple of integers corresponding to the start and end
/// of the match. The indices are byte offsets. The iterator stops when no more
/// matches can be found.
///
/// `'r` is the lifetime of the compiled expression and `'t` is the lifetime
/// of the matched string.
pub struct FindMatches<'r, 't> {
    re: &'r Regexp,
    search: &'t str,
    last_match: Option<uint>,
    last_end: uint,
}

impl<'r, 't> Iterator<(uint, uint)> for FindMatches<'r, 't> {
    fn next(&mut self) -> Option<(uint, uint)> {
        if self.last_end > self.search.len() {
            return None
        }

        let caps = exec_slice(self.re, Location, self.search,
                              self.last_end, self.search.len());
        let (s, e) =
            if !has_match(&caps) {
                return None
            } else {
                (caps.get(0).unwrap(), caps.get(1).unwrap())
            };

        // Don't accept empty matches immediately following a match.
        // i.e., no infinite loops please.
        if e - s == 0 && Some(self.last_end) == self.last_match {
            self.last_end += 1;
            return self.next()
        }
        self.last_end = e;
        self.last_match = Some(self.last_end);
        Some((s, e))
    }
}

fn exec(re: &Regexp, which: MatchKind, input: &str) -> CaptureLocs {
    exec_slice(re, which, input, 0, input.len())
}

fn exec_slice(re: &Regexp, which: MatchKind,
              input: &str, s: uint, e: uint) -> CaptureLocs {
    match re.p {
        Dynamic(ref prog) => vm::run(which, prog, input, s, e),
        Native(exec) => exec(which, input, s, e),
    }
}

#[inline(always)]
fn has_match(caps: &CaptureLocs) -> bool {
    caps.len() >= 2 && caps.get(0).is_some() && caps.get(1).is_some()
}
