use collections::HashMap;
use std::from_str::from_str;
use std::str;

use super::compile::Program;
use super::parse::{parse, Error};
use super::vm;
use super::vm::CapturePairs;

/// Regexp is a compiled regular expression. It can be used to search, split
/// or replace text.
///
/// # Examples
///
/// Find the location of a phone number:
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
/// You can also use the `re!` macro to compile a regular expression when
/// you compile your program:
///
/// ```rust
/// #![feature(phase)]
/// extern crate regexp;
/// #[phase(syntax)] extern crate regexp_re;
///
/// fn main() {
///     static re: regexp::Regexp = re!(r"\d+");
///     assert_eq!(re.find("123 abc"), Some((0, 3)));
/// }
/// ```
///
/// `re!` can also be declared with `static` in a module scope.
/// Given an incorrect regular expression, `re!` will cause the Rust compiler 
/// to produce a compile time error.
/// More details about the `re!` macro can be found in the `regexp` crate
/// documentation.
pub struct Regexp {
    /// The representation of `Regexp` is exported to support the `re!`
    /// syntax extension. Do not rely on it.
    ///
    /// See the comments for the `program` module in `lib.rs` for a more
    /// detailed explanation for what `re!` requires.
    #[doc(hidden)]
    pub p: Program,
}

impl Regexp {
    /// Creates a new compiled regular expression. Once compiled, it can be
    /// used repeatedly to search, split or replace text in a string.
    pub fn new(regex: &str) -> Result<Regexp, Error> {
        let ast = try!(parse(regex));
        Ok(Regexp { p: Program::new(regex, ast) })
    }

    /// Returns true if and only if the regexp matches the string given.
    pub fn is_match(&self, text: &str) -> bool {
        has_match(&SearchText::from_str(text, false).exec(self))
    }

    /// Returns the start and end byte range of the leftmost-longest match in 
    /// `text`. If no match exists, then `None` is returned.
    ///
    /// Note that this should only be used if you want to discover the position
    /// of the match. Testing the existence of a match is faster if you use
    /// `is_match`.
    pub fn find(&self, text: &str) -> Option<(uint, uint)> {
        let search = SearchText::from_str(text, true);
        *search.exec(self).get(0)
    }

    /// Iterates through each successive non-overlapping match in `text`,
    /// returning the start and end byte indices with respect to `text`.
    pub fn find_iter<'r, 't>(&'r self, text: &'t str) -> FindMatches<'r, 't> {
        FindMatches {
            re: self,
            search: SearchText::from_str(text, true),
            last_end: 0,
            last_match: 0,
        }
    }

    /// Returns the capture groups corresponding to the leftmost-longest
    /// match in `text`. Capture group `0` always corresponds to the entire 
    /// match. If no match is found, then `None` is returned.
    pub fn captures<'t>(&self, text: &'t str) -> Option<Captures<'t>> {
        let search = SearchText::from_str(text, true);
        let caps = search.exec(self);
        Captures::new(self, &search, caps)
    }

    /// Returns an iterator over all the non-overlapping capture groups matched
    /// in `text`. This is operationally the same as `find_iter` (except it
    /// yields capture groups and not positions).
    pub fn captures_iter<'r, 't>(&'r self, text: &'t str) -> FindCaptures<'r, 't> {
        FindCaptures {
            re: self,
            search: SearchText::from_str(text, true),
            last_match: 0,
            last_end: 0,
        }
    }

    /// Returns an iterator of substrings of `text` delimited by a match
    /// of the regular expression.
    /// Namely, each element of the iterator corresponds to text that *isn't* 
    /// matched by the regular expression.
    pub fn split<'r, 't>(&'r self, text: &'t str) -> RegexpSplits<'r, 't> {
        RegexpSplits {
            finder: self.find_iter(text),
            text: text,
            last: 0,
        }
    }

    /// Returns an iterator of `limit` substrings of `text` delimited by a 
    /// match of the regular expression. (A `limit` of `0` will return no
    /// substrings.)
    /// Namely, each element of the iterator corresponds to text that *isn't* 
    /// matched by the regular expression.
    pub fn splitn<'r, 't>(&'r self, text: &'t str, limit: uint)
                         -> RegexpSplitsN<'r, 't> {
        RegexpSplitsN {
            splits: self.split(text),
            cur: 0,
            limit: limit,
        }
    }

    /// Replaces the leftmost-longest match with the replacement provided.
    /// The replacement can be a regular string (where `$N` and `$name` are
    /// expanded to match capture groups) or a function that takes the matches' 
    /// `Captures` and returns the replaced string.
    ///
    /// If no match is found, then a copy of the string is returned unchanged.
    ///
    /// Note that this function in polymorphic with respect to the replacement.
    /// In typical usage, this can just be a normal string:
    ///
    /// ```rust
    /// # #![feature(phase)]
    /// # extern crate regexp; #[phase(syntax)] extern crate regexp_re;
    /// # use regexp::Regexp; fn main() {
    /// static re: Regexp = re!("[^01]+");
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
    /// # extern crate regexp; #[phase(syntax)] extern crate regexp_re;
    /// # use regexp::{Regexp, Captures}; fn main() {
    /// static re: Regexp = re!(r"([^,\s]+),\s+(\S+)");
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
    /// # extern crate regexp; #[phase(syntax)] extern crate regexp_re;
    /// # use regexp::Regexp; fn main() {
    /// static re: Regexp = re!(r"(?P<last>[^,\s]+),\s+(?P<first>\S+)");
    /// let result = re.replace("Springsteen, Bruce", "$first $last");
    /// assert_eq!(result, ~"Bruce Springsteen");
    /// # }
    /// ```
    ///
    /// Note that using `$2` instead of `$first` or `$1` instead of `$last`
    /// would produce the same result. To write a literal `$` use `$$`.
    ///
    /// Finally, sometimes you just want to replace a literal string with no
    /// fancy submatch expansion. This can be done by wraping a string with
    /// `NoExpand`:
    ///
    /// ```rust
    /// # #![feature(phase)]
    /// # extern crate regexp; #[phase(syntax)] extern crate regexp_re;
    /// # use regexp::{Regexp, NoExpand}; fn main() {
    /// static re: Regexp = re!(r"(?P<last>[^,\s]+),\s+(\S+)");
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
        let mut new = str::with_capacity(text.len());
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
            new.push_str(text.slice(last_match, s));
            new.push_str(rep.reg_replace(&cap));
            last_match = e;
        }
        new.push_str(text.slice(last_match, text.len()));
        new
    }
}

/// NoExpand indicates literal string replacement.
///
/// It can be used with `replace` and `replace_all` to do a literal
/// string replacement without expanding `$name` to their corresponding
/// capture groups.
pub struct NoExpand<'r>(pub &'r str);

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
pub fn expand(caps: &Captures, text: &str) -> ~str {
    // How evil can you get?
    // FIXME: Don't use regexes for this. It's completely unnecessary.
    // FIXME: Marginal improvement: get a syntax extension re! to prevent
    //        recompilation every time.
    let re = Regexp::new(r"(^|[^$])\$(\w+)").unwrap();
    let text = re.replace_all(text, |refs: &Captures| -> ~str {
        let (pre, name) = (refs.at(1), refs.at(2));
        pre + match from_str::<uint>(name) {
            None => caps.name(name).to_owned(),
            Some(i) => caps.at(i).to_owned(),
        }
    });
    text.replace("$$", "$")
}

/// Replacer describes types that can be used to replace matches in a string.
pub trait Replacer {
    fn reg_replace(&self, caps: &Captures) -> ~str;
}

impl<'r> Replacer for NoExpand<'r> {
    fn reg_replace(&self, _: &Captures) -> ~str {
        let NoExpand(s) = *self;
        s.to_owned()
    }
}

impl<'r> Replacer for &'r str {
    fn reg_replace(&self, caps: &Captures) -> ~str {
        expand(caps, *self)
    }
}

impl<'r> Replacer for |&Captures|: 'r -> ~str {
    fn reg_replace(&self, caps: &Captures) -> ~str {
        (*self)(caps)
    }
}

/// Yields all substrings delimited by a regular expression match.
pub struct RegexpSplits<'r, 't> {
    finder: FindMatches<'r, 't>,
    text: &'t str,
    last: uint,
}

impl<'r, 't> Iterator<&'t str> for RegexpSplits<'r, 't> {
    fn next(&mut self) -> Option<&'t str> {
        match self.finder.next() {
            None => {
                if self.last >= self.text.len() {
                    None
                } else {
                    let s = self.text.slice(self.last, self.text.len());
                    self.last = self.text.len();
                    Some(s)
                }
            }
            Some((s, e)) => {
                let text = self.text.slice(self.last, s);
                self.last = e;
                Some(text)
            }
        }
    }
}

/// Yields at most `N` substrings delimited by a regular expression match.
///
/// The last substring will be whatever remains after splitting.
pub struct RegexpSplitsN<'r, 't> {
    splits: RegexpSplits<'r, 't>,
    cur: uint,
    limit: uint,
}

impl<'r, 't> Iterator<&'t str> for RegexpSplitsN<'r, 't> {
    fn next(&mut self) -> Option<&'t str> {
        if self.cur >= self.limit {
            None
        } else {
            self.cur += 1;
            if self.cur >= self.limit {
                Some(self.splits.text.slice(self.splits.last,
                                            self.splits.text.len()))
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
pub struct Captures<'t> {
    text: &'t str,
    locs: CapturePairs,
    named: HashMap<~str, uint>,
    offset: uint,
}

impl<'t> Captures<'t> {
    fn new(re: &Regexp, search: &SearchText<'t>,
           locs: CapturePairs) -> Option<Captures<'t>> {
        if !has_match(&locs) {
            return None
        }

        let mut named = HashMap::new();
        for (i, name) in re.p.names.as_slice().iter().enumerate() {
            match name {
                &None => {},
                &Some(ref name) => {
                    named.insert(name.as_slice().to_owned(), i);
                }
            }
        }
        Some(Captures {
            text: search.text,
            locs: locs,
            named: named,
            offset: 0,
        })
    }

    /// Returns the start and end positions of the Nth capture group.
    /// Returns `(0, 0)` if `i` is not a valid capture group.
    /// The positions returned are *always* byte indices with respect to the 
    /// original string matched.
    pub fn pos(&self, i: uint) -> Option<(uint, uint)> {
        if i >= self.locs.len() {
            return None
        }
        *self.locs.get(i)
    }

    /// Returns the matched string for the capture group `i`.
    /// If `i` isn't a valid capture group, then the empty string is returned.
    pub fn at(&self, i: uint) -> &'t str {
        match self.pos(i) {
            None => "",
            Some((s, e)) => {
                self.text.slice(s, e)
            }
        }
    }

    /// Returns the matched string for the capture group named `name`.
    /// If `name` isn't a valid capture group, then the empty string is 
    /// returned.
    pub fn name(&self, name: &str) -> &'t str {
        match self.named.find(&name.to_owned()) {
            None => "",
            Some(i) => self.at(*i),
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
}

impl<'t> Container for Captures<'t> {
    fn len(&self) -> uint {
        self.locs.len()
    }
}

/// An iterator over capture groups for a particular match of a regular
/// expression.
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
/// particular regular expression.
pub struct FindCaptures<'r, 't> {
    re: &'r Regexp,
    search: SearchText<'t>,
    last_match: uint,
    last_end: uint,
}

impl<'r, 't> Iterator<Captures<'t>> for FindCaptures<'r, 't> {
    fn next(&mut self) -> Option<Captures<'t>> {
        if self.last_end > self.search.chars.len() {
            return None
        }

        let uni_caps = self.search.exec_slice(self.re,
                                              self.last_end,
                                              self.search.chars.len());
        let (us, ue) =
            if !has_match(&uni_caps) {
                return None
            } else {
                uni_caps.get(0).unwrap()
            };
        let char_len = ue - us;

        // Don't accept empty matches immediately following a match.
        // i.e., no infinite loops please.
        if char_len == 0 && self.last_end == self.last_match {
            self.last_end += 1;
            return self.next()
        }

        let bytei = self.search.bytei.as_slice();
        let byte_caps = cap_to_byte_indices(uni_caps, bytei);
        let caps = Captures::new(self.re, &self.search, byte_caps);

        self.last_end = ue;
        self.last_match = self.last_end;
        caps
    }
}

/// An iterator over all non-overlapping matches for a particular string.
///
/// The iterator yields a tuple of integers corresponding to the start and end
/// of the match. The indices are byte offsets.
pub struct FindMatches<'r, 't> {
    re: &'r Regexp,
    search: SearchText<'t>,
    last_match: uint,
    last_end: uint,
}

impl<'r, 't> Iterator<(uint, uint)> for FindMatches<'r, 't> {
    fn next(&mut self) -> Option<(uint, uint)> {
        if self.last_end > self.search.chars.len() {
            return None
        }

        let uni_caps = self.search.exec_slice(self.re,
                                              self.last_end,
                                              self.search.chars.len());
        let (us, ue) =
            if !has_match(&uni_caps) {
                return None
            } else {
                uni_caps.get(0).unwrap()
            };
        let char_len = ue - us;

        // Don't accept empty matches immediately following a match.
        // i.e., no infinite loops please.
        if char_len == 0 && self.last_end == self.last_match {
            self.last_end += 1;
            return self.next()
        }

        self.last_end = ue;
        self.last_match = self.last_end;
        Some((*self.search.bytei.get(us), *self.search.bytei.get(ue)))
    }
}

struct SearchText<'t> {
    text: &'t str,
    chars: Vec<char>,
    bytei: Vec<uint>,
    caps: bool,
}

// TODO: Choose better names. There's some complicated footwork going on here
// to handle character and byte indices.
impl<'t> SearchText<'t> {
    fn from_str(input: &'t str, caps: bool) -> SearchText<'t> {
        let chars = input.chars().collect();
        let bytei = char_to_byte_indices(input);
        SearchText { text: input, chars: chars, bytei: bytei, caps: caps }
    }

    fn exec(&self, re: &Regexp) -> CapturePairs {
        let caps = vm::run(&re.p, self.chars.as_slice(), self.caps);
        cap_to_byte_indices(caps, self.bytei.as_slice())
    }

    fn exec_slice(&self, re: &Regexp, us: uint, ue: uint) -> CapturePairs {
        let chars = self.chars.as_slice().slice(us, ue);
        let caps = vm::run(&re.p, chars, self.caps);
        caps.iter().map(|loc| loc.map(|(s, e)| (us + s, us + e))).collect()
    }
}

impl<'t> Container for SearchText<'t> {
    fn len(&self) -> uint {
        self.text.len()
    }
}

fn cap_to_byte_indices(mut cis: CapturePairs, bis: &[uint])
                      -> CapturePairs {
    for v in cis.mut_iter() {
        *v = v.map(|(s, e)| (bis[s], bis[e]))
    }
    cis
}

fn char_to_byte_indices(input: &str) -> Vec<uint> {
    let mut bytei = Vec::with_capacity(input.len());
    for (bi, _) in input.char_indices() {
        bytei.push(bi);
    }
    // Push one more for the length.
    bytei.push(input.len());
    bytei
}

fn has_match(caps: &CapturePairs) -> bool {
    caps.len() > 0 && caps.get(0).is_some()
}
