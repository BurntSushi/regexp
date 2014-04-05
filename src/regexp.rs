use collections::HashMap;
use std::from_str::from_str;
use std::slice;
use std::str;

use super::Error;
use super::compile::{Inst, compile};
use super::parse::parse;
use super::vm::{CaptureIndices, run};

/// Regexp is a compiled regular expression.
pub struct Regexp {
    orig: ~str,
    prog: Vec<Inst>,
    names: Vec<Option<~str>>,
}

fn to_byte_indices(s: &str, ulocs: CaptureIndices) -> CaptureIndices {
    // FIXME: This seems incredibly slow and unfortunate and I think it can
    // be removed completely.
    // I wonder if there is a way to get the VM to return byte indices easily.
    // Preferably if it can be done without disrupting the fact that everything
    // works at the Unicode `char` granularity.
    // (Maybe keep track of byte index as we move through string?)

    let mut blocs = Vec::from_elem(ulocs.len(), (0u, 0u));
    let biggest = ulocs.get(0).val1(); // first capture is always biggest
    for (s_uloc, (bloc, _)) in s.char_indices().enumerate() {
        if s_uloc > biggest {
            // We can stop processing the string once we know we're done
            // mapping to byte indices.
            break
        }
        for (loci, &(suloc, euloc)) in ulocs.iter().enumerate() {
            if suloc == s_uloc {
                *blocs.get_mut(loci).mut0() = bloc;
            }
            if euloc == s_uloc {
                *blocs.get_mut(loci).mut1() = bloc;
            }
        }
    }
    // We also need to make sure that ending positions that correspond to
    // the character length of 's' are mapped to the byte length.
    let char_len = s.char_len();
    for (loci, &(suloc, euloc)) in ulocs.iter().enumerate() {
        if suloc == char_len {
            *blocs.get_mut(loci).mut0() = s.len();
        }
        if euloc == char_len {
            *blocs.get_mut(loci).mut1() = s.len();
        }
    }
    blocs
}

impl Regexp {
    /// Creates a new compiled regular expression. Once compiled, it can be
    /// used repeatedly to search, split or replace text in a string.
    pub fn new(s: &str) -> Result<Regexp, Error> {
        let ast = try!(parse(s));
        let (insts, cap_names) = compile(ast);
        Ok(Regexp {
            orig: s.to_owned(),
            prog: insts,
            names: cap_names,
        })
    }

    /// Executes the VM on the string given and converts the positions
    /// returned from Unicode character indices to byte indices.
    fn run(&self, s: &str) -> Option<CaptureIndices> {
        run(self.prog.as_slice(), s).map(|ulocs| to_byte_indices(s, ulocs))
        // vm::run(self.prog.as_slice(), s) 
    }

    /// Returns true if and only if the regexp matches the string given.
    pub fn is_match(&self, s: &str) -> bool {
        self.run(s).is_some()
    }

    /// Returns the start and end byte range of the leftmost-longest match in 
    /// `s`. If no match exists, then `None` is returned.
    pub fn find(&self, s: &str) -> Option<(uint, uint)> {
        self.run(s).map(|locs| *locs.get(0))
    }

    /// Iterates through each successive non-overlapping match in `s`, 
    /// returning the start and end byte indices with respect to `s`.
    pub fn find_iter<'r>(&'r self, s: &str) -> FindMatches<'r> {
        FindMatches {
            re: self,
            text: s.to_owned(),
            last_end: 0,
            last_match: 0,
        }
    }

    /// Returns the capture groups corresponding to the leftmost-longest
    /// match in `text`. Capture group `0` always corresponds to the entire 
    /// match. If no match is found, then `None` is returned.
    pub fn captures(&self, text: &str) -> Option<Captures> {
        let locs =
            match self.run(text) {
                None => return None,
                Some(locs) => locs,
            };
        let &(_, e) = locs.get(0);
        let max_match = text.slice(0, e).to_owned();
        Some(Captures::from_locs(max_match, self.names.as_slice(), locs))
    }

    /// Returns an iterator over all the non-overlapping capture groups matched
    /// in `text`. This is operationally the same as `find_iter` (except it
    /// yields capture groups and not positions).
    pub fn captures_iter<'r>(&'r self, text: &str) -> FindCaptures<'r> {
        FindCaptures {
            re: self,
            text: text.to_owned(),
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
    /// expanded to match capture groups) or a function that takes the matche's
    /// `Captures` and returns the replaced string.
    ///
    /// If no match is found, then a copy of the string is returned unchanged.
    pub fn replace<R: Replacer>(&self, text: &str, rep: R) -> ~str {
        let caps =
            match self.captures(text) {
                None => return ~"",
                Some(caps) => caps,
            };
        let mut new = str::with_capacity(text.len());
        let (s, e) = caps.pos(0);
        new.push_str(text.slice(0, s));
        new.push_str(rep.replace(&caps));
        new.push_str(text.slice(e, text.len()));
        new
    }

    /// Replaces all non-overlapping matches in `text` with the replacement
    /// provided.
    pub fn replace_all<R: Replacer>(&self, text: &str, rep: R) -> ~str {
        let mut new = str::with_capacity(text.len());
        let mut last_match = 0u;
        for cap in self.captures_iter(text) {
            let (s, e) = cap.pos(0);
            new.push_str(text.slice(last_match, s));
            new.push_str(rep.replace(&cap));
            last_match = e;
        }
        new.push_str(text.slice(last_match, text.len()));
        new
    }
}

/// NoExpand can be used with `replace` and `replace_all` to do a literal
/// string replacement without expanding `$name` to their corresponding
/// capture groups.
pub struct NoExpand<'r>(pub &'r str);

/// Expands all instances of `$name` in `text` to the corresponding capture
/// group `name`. `name` may be an integer corresponding to the index of the
/// capture group (counted by order of opening parenthesis where `0` is the
/// entire match) or it can be a name (consisting of letters, digits or 
/// underscores) corresponding to a named capture group.
pub fn expand(caps: &Captures, text: &str) -> ~str {
    // How evil can you get?
    // FIXME: Don't use regexes for this. It's completely unnecessary.
    // FIXME: Marginal improvement: get a syntax extension re! to prevent
    //        recompilation every time.
    let re = Regexp::new(r"(^|[^$])\$(\w+)").unwrap();
    re.replace_all(text, |refs: &Captures| -> ~str {
        let (pre, name) = (refs.at(1), refs.at(2));
        pre + match from_str::<uint>(name) {
            None => caps.name(name).to_owned(),
            Some(i) => caps.at(i).to_owned(),
        }
    })
}

/// Replacer describes types that can be used to replace matches in a string.
trait Replacer {
    fn replace(&self, caps: &Captures) -> ~str;
}

impl<'r> Replacer for NoExpand<'r> {
    fn replace(&self, _: &Captures) -> ~str {
        let NoExpand(s) = *self;
        s.to_owned()
    }
}

impl<'r> Replacer for &'r str {
    fn replace(&self, caps: &Captures) -> ~str {
        expand(caps, *self)
    }
}

impl<'r> Replacer for 'r |&Captures| -> ~str {
    fn replace(&self, caps: &Captures) -> ~str {
        (*self)(caps)
    }
}

/// Yields all substrings delimited by a regular expression match.
pub struct RegexpSplits<'r, 't> {
    finder: FindMatches<'r>,
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
pub struct Captures {
    max_match: ~str,
    locs: CaptureIndices,
    named: HashMap<~str, uint>,
    offset: uint,
}

impl Captures {
    /// Creates a new group of captures from the matched string, a list of
    /// capture names and a list of locations.
    fn from_locs(s: ~str, names: &[Option<~str>],
                 locs: CaptureIndices) -> Captures {
        let mut named = HashMap::new();
        for (i, name) in names.iter().enumerate() {
            match name {
                &None => {},
                &Some(ref name) => {
                    named.insert(name.to_owned(), i);
                }
            }
        }
        Captures {
            max_match: s,
            locs: locs,
            named: named,
            offset: 0,
        }
    }

    /// Adds offset to each location in the captures so that `pos` always
    /// returns byte indices in the original string.
    fn adjust_locations(&mut self, offset: uint) {
        self.offset = offset;
    }

    /// Returns the matched string for the capture group `i`.
    /// If `i` isn't a valid capture group, then the empty string is returned.
    pub fn at<'r>(&'r self, i: uint) -> &'r str {
        if i >= self.locs.len() {
            return ""
        }
        let &(s, e) = self.locs.get(i);
        self.max_match.slice(s, e)
    }

    /// Returns the matched string for the capture group named `name`.
    /// If `name` isn't a valid capture group, then the empty string is 
    /// returned.
    pub fn name<'r>(&'r self, name: &str) -> &'r str {
        match self.named.find(&name.to_owned()) {
            None => "",
            Some(i) => self.at(*i),
        }
    }

    /// Returns the start and end positions of the Nth capture group.
    /// Returns `(0, 0)` if `i` is not a valid capture group.
    /// The positions returned are *always* byte indices with respect to the 
    /// original string matched.
    pub fn pos(&self, i: uint) -> (uint, uint) {
        if i >= self.locs.len() {
            return (0u, 0u)
        }
        let (s, e) = *self.locs.get(i);
        (s + self.offset, e + self.offset)
    }

    /// Creates an iterator of all the capture groups in order of appearance
    /// in the regular expression.
    pub fn iter<'r>(&'r self) -> SubCaptures<'r> {
        SubCaptures { idx: 0, caps: self, }
    }
}

impl Container for Captures {
    fn len(&self) -> uint {
        self.locs.len()
    }
}

/// An iterator over capture groups for a particular match of a regular
/// expression.
pub struct SubCaptures<'r> {
    idx: uint,
    caps: &'r Captures,
}

impl<'r> Iterator<&'r str> for SubCaptures<'r> {
    fn next(&mut self) -> Option<&'r str> {
        if self.idx < self.caps.len() {
            self.idx += 1;
            Some(self.caps.at(self.idx - 1))
        } else {
            None
        }
    }
}

/// An iterator that yields all non-overlapping capture groups matching a
/// particular regular expression.
pub struct FindCaptures<'r> {
    re: &'r Regexp,
    text: ~str,
    last_match: uint,
    last_end: uint,
}

impl<'r> Iterator<Captures> for FindCaptures<'r> {
    fn next(&mut self) -> Option<Captures> {
        if self.last_end > self.text.len() {
            return None
        }
        let caps = {
            let t = self.text.slice(self.last_end, self.text.len());
            self.re.captures(t)
        };
        match caps {
            None => None,
            Some(mut caps) => {
                caps.adjust_locations(self.last_end);

                // Don't accept empty matches immediately following a match.
                // i.e., no infinite loops please.
                if caps.at(0).len() == 0 && self.last_end == self.last_match {
                    self.last_end += 1;
                    return self.next()
                }
                self.last_end += caps.max_match.len();
                self.last_match = self.last_end;
                Some(caps)
            }
        }
    }
}

/// An iterator over all non-overlapping matches for a particular string.
///
/// The iterator yields a tuple of integers corresponding to the start and end
/// of the match. The indices are byte offsets.
pub struct FindMatches<'r> {
    re: &'r Regexp,
    text: ~str,
    last_match: uint,
    last_end: uint,
}

impl<'r> Iterator<(uint, uint)> for FindMatches<'r> {
    fn next(&mut self) -> Option<(uint, uint)> {
        if self.last_end > self.text.len() {
            return None
        }
        let find = {
            let t = self.text.slice(self.last_end, self.text.len());
            self.re.find(t)
        };
        match find {
            None => None,
            Some((mut s, mut e)) => {
                s += self.last_end;
                e += self.last_end;

                // Don't accept empty matches immediately following a match.
                // i.e., no infinite loops please.
                if self.last_end == e && self.last_end == self.last_match {
                    self.last_end += 1;
                    return self.next()
                }
                self.last_end = e;
                self.last_match = self.last_end;
                Some((s, e))
            }
        }
    }
}
