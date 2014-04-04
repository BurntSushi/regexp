use collections::HashMap;
use std::slice;

use super::Error;
use super::compile::{Inst, compile};
use super::parse::parse;
use super::vm;

pub struct Regexp {
    orig: ~str,
    prog: Vec<Inst>,
    names: Vec<Option<~str>>,
}

pub fn to_byte_indices(s: &str, ulocs: Vec<uint>) -> Vec<uint> {
    // FIXME: This seems incredibly slow and unfortunate and I think it can
    // be removed completely.
    // I wonder if there is a way to get the VM to return byte indices easily.
    // Preferably if it can be done without disrupting the fact that everything
    // works at the Unicode `char` granularity.
    // (Maybe keep track of byte index as we move through string?)

    let mut blocs = Vec::from_elem(ulocs.len(), 0u);
    let biggest = *ulocs.get(1); // first capture is always biggest
    for (s_uloc, (bloc, _)) in s.char_indices().enumerate() {
        if s_uloc > biggest {
            // We can stop processing the string once we know we're done
            // mapping to byte indices.
            break
        }
        for (loci, &uloc) in ulocs.iter().enumerate() {
            if uloc == s_uloc {
                *blocs.get_mut(loci) = bloc;
            }
        }
    }
    // We also need to make sure that ending positions that correspond to
    // the character length of 's' are mapped to the byte length.
    let char_len = s.char_len();
    for (loci, &uloc) in ulocs.iter().enumerate() {
        if uloc == char_len {
            *blocs.get_mut(loci) = s.len();
        }
    }
    blocs
}

impl Regexp {
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
    fn run(&self, s: &str) -> Option<Vec<uint>> {
        vm::run(self.prog.as_slice(), s).map(|ulocs| to_byte_indices(s, ulocs))
        // vm::run(self.prog.as_slice(), s) 
    }

    /// Returns true if and only if the regexp matches the string given.
    pub fn is_match(&self, s: &str) -> bool {
        self.run(s).is_some()
    }

    /// Returns the start and end byte range of the leftmost-longest match in 
    /// `s`. If no match exists, then `None` is returned.
    pub fn find(&self, s: &str) -> Option<(uint, uint)> {
        self.run(s).map(|locs| (*locs.get(0), *locs.get(1)))
    }

    /// Iterates through each successive non-overlapping match in `s`, 
    /// returning the start and end byte indices with respect to `s`.
    pub fn find_iter<'r>(&'r self, s: &str) -> FindMatches<'r> {
        FindMatches {
            re: self,
            text: s.to_owned(),
            byte_len: s.len(),
            last_end: 0,
        }
    }

    /// Returns the capture groups corresponding to the leftmost-longest
    /// match in `s`. Capture group `0` always corresponds to the entire match.
    /// If no match is found, then `None` is returned.
    pub fn captures(&self, s: &str) -> Option<Captures> {
        let locs =
            match self.run(s) {
                None => return None,
                Some(locs) => locs,
            };
        let full_match = s.slice(*locs.get(0), *locs.get(1)).to_owned();
        Some(Captures::from_locs(full_match, self.names.as_slice(), locs))
    }
}

pub struct Captures {
    full_match: ~str,
    locs: Vec<uint>,
    named: HashMap<~str, uint>,
}

impl Captures {
    fn from_locs(s: ~str, names: &[Option<~str>], locs: Vec<uint>) -> Captures {
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
            full_match: s,
            locs: locs,
            named: named,
        }
    }

    pub fn at<'r>(&'r self, i: uint) -> &'r str {
        let k = 2 * i;
        self.full_match.slice(*self.locs.get(k), *self.locs.get(k+1))
    }

    pub fn name<'r>(&'r self, name: &str) -> &'r str {
        match self.named.find(&name.to_owned()) {
            None => "",
            Some(i) => self.at(*i),
        }
    }

    pub fn subs<'r>(&'r self) -> SubCaptures<'r> {
        SubCaptures { idx: 0, caps: self, }
    }
}

impl Container for Captures {
    fn len(&self) -> uint {
        self.locs.len() / 2
    }
}

struct SubCaptures<'r> {
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

pub struct FindMatches<'r> {
    re: &'r Regexp,
    text: ~str,
    byte_len: uint,
    last_end: uint,
}

impl<'r> Iterator<(uint, uint)> for FindMatches<'r> {
    fn next(&mut self) -> Option<(uint, uint)> {
        let t = self.text.slice(self.last_end, self.byte_len);
        match self.re.find(t) {
            None => None,
            Some((mut s, mut e)) => {
                s += self.last_end;
                e += self.last_end;
                self.last_end = e;
                Some((s, e))
            }
        }
    }
}
