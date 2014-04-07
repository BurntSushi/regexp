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

impl Regexp {
    /// Creates a new compiled regular expression. Once compiled, it can be
    /// used repeatedly to search, split or replace text in a string.
    pub fn new(regex: &str) -> Result<Regexp, Error> {
        let ast = try!(parse(regex));
        let (insts, cap_names) = compile(ast);
        Ok(Regexp {
            orig: regex.to_owned(),
            prog: insts,
            names: cap_names,
        })
    }

    /// Executes the VM on the string given and converts the positions
    /// returned from Unicode character indices to byte indices.
    fn run(&self, text: &str) -> CaptureIndices {
        run(self.prog.as_slice(), text)
    }

    /// Returns true if and only if the regexp matches the string given.
    pub fn is_match(&self, text: &str) -> bool {
        self.has_match(self.run(text))
    }

    fn has_match(&self, caps: CaptureIndices) -> bool {
        caps.len() > 0 && caps.get(0).is_some()
    }

    /// Returns the start and end byte range of the leftmost-longest match in 
    /// `text`. If no match exists, then `None` is returned.
    pub fn find(&self, text: &str) -> Option<(uint, uint)> {
        *self.run(text).get(0)
    }

    /// Iterates through each successive non-overlapping match in `text`,
    /// returning the start and end byte indices with respect to `text`.
    pub fn find_iter<'r>(&'r self, text: &str) -> FindMatches<'r> {
        FindMatches {
            re: self,
            text: text.to_owned(),
            last_end: 0,
            last_match: 0,
        }
    }

    /// Returns the capture groups corresponding to the leftmost-longest
    /// match in `text`. Capture group `0` always corresponds to the entire 
    /// match. If no match is found, then `None` is returned.
    pub fn captures<'r>(&self, text: &'r str) -> Option<Captures<'r>> {
        let locs = self.run(text);
        let end = match *locs.get(0) {
            None => return None,
            Some((_, e)) => e,
        };
        let max_match = text.slice(0, end);
        Some(Captures::from_locs(max_match, self.names.as_slice(), locs))
    }

    /// Returns an iterator over all the non-overlapping capture groups matched
    /// in `text`. This is operationally the same as `find_iter` (except it
    /// yields capture groups and not positions).
    pub fn captures_iter<'r>(&'r self, text: &'r str) -> FindCaptures<'r> {
        FindCaptures {
            re: self,
            text: text,
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
    pub fn replace<R: Replacer>(&self, text: &str, rep: R) -> ~str {
        let caps =
            match self.captures(text) {
                None => return ~"",
                Some(caps) => caps,
            };
        let (s, e) = match caps.pos(0) {
            None => return text.to_owned(), // hmm, switch to MaybeOwned?
            Some((s, e)) => (s, e),
        };
        let mut new = str::with_capacity(text.len());
        new.push_str(text.slice(0, s));
        new.push_str(rep.replace(&caps));
        new.push_str(text.slice(e, text.len()));
        new
    }

    /// Replaces all non-overlapping matches in `text` with the 
    /// replacement provided. This is the same as calling `replacen` with
    /// `limit` set to `0`.
    pub fn replace_all<R: Replacer>(&self, text: &str, rep: R) -> ~str {
        self.replacen(text, 0, rep)
    }

    /// Replaces at most `limit` non-overlapping matches in `text` with the 
    /// replacement provided. If `limit` is 0, then all non-overlapping matches
    /// are replaced.
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
            new.push_str(rep.replace(&cap));
            last_match = e;
        }
        println!("REPLACED: {}", i);
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
    re.replace_all(text, |refs: &Captures| -> ~str {
        let (pre, name) = (refs.at(1), refs.at(2));
        pre + match from_str::<uint>(name) {
            None => caps.name(name).to_owned(),
            Some(i) => caps.at(i).to_owned(),
        }
    })
}

/// Replacer describes types that can be used to replace matches in a string.
pub trait Replacer {
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
pub struct Captures<'r> {
    max_match: &'r str,
    locs: CaptureIndices,
    named: HashMap<~str, uint>,
    offset: uint,
}

impl<'r> Captures<'r> {
    /// Creates a new group of captures from the matched string, a list of
    /// capture names and a list of locations.
    fn from_locs(s: &'r str, names: &[Option<~str>],
                 locs: CaptureIndices) -> Captures<'r> {
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
    pub fn at(&self, i: uint) -> &'r str {
        // We're not reusing the 'pos' method here since 'pos' reports offsets
        // in terms of the original matched string.
        if i >= self.locs.len() {
            return ""
        }
        match *self.locs.get(i) {
            None => "",
            Some((s, e)) => self.max_match.slice(s, e),
        }
    }

    /// Returns the matched string for the capture group named `name`.
    /// If `name` isn't a valid capture group, then the empty string is 
    /// returned.
    pub fn name(&self, name: &str) -> &'r str {
        match self.named.find(&name.to_owned()) {
            None => "",
            Some(i) => self.at(*i),
        }
    }

    /// Returns the start and end positions of the Nth capture group.
    /// Returns `(0, 0)` if `i` is not a valid capture group.
    /// The positions returned are *always* byte indices with respect to the 
    /// original string matched.
    pub fn pos(&self, i: uint) -> Option<(uint, uint)> {
        if i >= self.locs.len() {
            return None
        }
        match *self.locs.get(i) {
            None => None,
            Some((s, e)) => Some((s + self.offset, e + self.offset)),
        }
    }

    /// Creates an iterator of all the capture groups in order of appearance
    /// in the regular expression.
    pub fn iter(&'r self) -> SubCaptures<'r> {
        SubCaptures { idx: 0, caps: self, }
    }

    /// Creates an iterator of all the capture group positions in order of 
    /// appearance in the regular expression. Positions are byte indices
    /// in terms of the original string matched.
    pub fn iter_pos(&'r self) -> SubCapturesPos<'r> {
        SubCapturesPos { idx: 0, caps: self, }
    }
}

impl<'r> Container for Captures<'r> {
    fn len(&self) -> uint {
        self.locs.len()
    }
}

/// An iterator over capture groups for a particular match of a regular
/// expression.
pub struct SubCaptures<'r> {
    idx: uint,
    caps: &'r Captures<'r>,
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

/// An iterator over capture group positions for a particular match of a 
/// regular expression.
///
/// Positions are byte indices in terms of the original string matched.
pub struct SubCapturesPos<'r> {
    idx: uint,
    caps: &'r Captures<'r>,
}

impl<'r> Iterator<Option<(uint, uint)>> for SubCapturesPos<'r> {
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
pub struct FindCaptures<'r> {
    re: &'r Regexp,
    text: &'r str,
    last_match: uint,
    last_end: uint,
}

impl<'r> Iterator<Captures<'r>> for FindCaptures<'r> {
    fn next(&mut self) -> Option<Captures<'r>> {
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
