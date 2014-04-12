// FIXME: Currently, the VM simulates an NFA. It would be nice to have another
// VM that simulates a DFA.
//
// According to Russ Cox[1], a DFA performs better than an NFA, principally
// because it reuses states previously computed by the machine *and* doesn't
// keep track of capture groups. The drawback of a DFA (aside from its 
// complexity) is that it can't accurately return the locations of submatches. 
// The NFA *can* do that. (This is my understanding anyway.)
//
// Cox suggests that a DFA ought to be used to answer "does this match" and
// "where does it match" questions. (In the latter, the starting position of
// the match is computed by executing the regexp backwards.) Cox also suggests
// that a DFA should be run when asking "where are the submatches", which can
// 1) quickly answer "no" is there's no match and 2) discover the substring
// that matches, which means running the NFA on smaller input.
//
// Currently, the NFA simulation implemented below does some dirty tricks to
// avoid tracking capture groups when they aren't needed (which only works
// for 'is_match', not 'find'). This is a half-measure, but does provide some
// perf improvement.
//
// AFAIK, the DFA/NFA approach is implemented in RE2/C++ but *not* in RE2/Go.

use std::cmp;
use std::mem;
use super::compile::{
    Program, Inst,
    Match, OneChar, CharClass, Any, EmptyBegin, EmptyEnd, EmptyWordBoundary,
    Save, Jump, Split,
};
use super::parse::{FLAG_NOCASE, FLAG_MULTI, FLAG_DOTNL, FLAG_NEGATED};

pub type CapturePairs = Vec<Option<(uint, uint)>>;
pub type CaptureLocs = Vec<Option<uint>>;

/// Runs an NFA simulation on the list of instructions and input given. (The
/// input must have been decoded into a slice of UTF8 characters.)
/// If 'caps' is true, then capture groups are tracked. When false, capture
/// groups (and 'Save' instructions) are ignored.
///
/// Note that if 'caps' is false, the capture indices returned will always be
/// one of two values: `vec!(None)` for no match or `vec!(Some((0, 0)))` for
/// a match.
pub fn run<'r, 't>(prog: &'r Program, input: &'t str, caps: bool,
                   start: uint, end: uint) -> CapturePairs {
    unflatten_capture_locations(Nfa {
        prog: prog,
        insts: prog.insts.as_slice(),
        input: input,
        caps: caps,
        start: start,
        end: end,
        ic: 0,
        prev: None,
        cur: None,
        next: None,
    }.run())
}

/// Converts the capture indices returned by a VM into tuples. It also makes
/// sure that the following invariant holds: for a particular capture group
/// k, the slots 2k and 2k+1 must both contain a location or must both be done
/// by the time the VM is done executing. (Otherwise there is a bug in the VM.)
fn unflatten_capture_locations(locs: CaptureLocs) -> CapturePairs {
    let mut caps = Vec::with_capacity(locs.len() / 2);
    for win in locs.as_slice().chunks(2) {
        match (win[0], win[1]) {
            (Some(s), Some(e)) => caps.push(Some((s, e))),
            (None, None) => caps.push(None),
            wins => fail!("BUG: Invalid capture group: {}", wins),
        }
    }
    caps
}

struct Nfa<'r, 't> {
    prog: &'r Program,
    insts: &'r [Inst],
    input: &'t str,
    caps: bool,
    start: uint,
    end: uint,
    ic: uint,
    prev: Option<char>,
    cur: Option<char>,
    next: Option<char>,
}

enum StepState {
    StepMatchEarlyReturn,
    StepMatch,
    StepContinue,
}

impl<'r, 't> Nfa<'r, 't> {
    fn run(&mut self) -> CaptureLocs {
        let num_caps = self.prog.num_captures();
        let clist = &mut Threads::new(self.insts.len(), num_caps);
        let nlist = &mut Threads::new(self.insts.len(), num_caps);

        let mut groups = Vec::from_elem(num_caps * 2, None);

        self.ic = self.start;
        let mut next_ic = self.set_chars(self.start);
        while self.ic <= self.end {
            if clist.size == 0 {
                // We have a match and we're done exploring alternatives.
                // Time to quit.
                if groups.get(0).is_some() {
                    break
                }

                // If there are no threads to try, then we'll have to start 
                // over at the beginning of the regex.
                // BUT, if there's a literal prefix for the program, try to 
                // jump ahead quickly. If it can't be found, then we can bail 
                // out early.
                if self.prog.prefix.len() > 0 && clist.size == 0 {
                    let needle = self.prog.prefix.as_slice().as_bytes();
                    let haystack = self.input.as_bytes().slice_from(self.ic);
                    match find_prefix(needle, haystack) {
                        None => return Vec::from_elem(num_caps * 2, None),
                        Some(i) => {
                            self.ic += i;
                            next_ic = self.set_chars(self.ic);
                            // println!("LITERAL MATCHED: {} :: {}", self.ic, i); 
                            // println!("needle: {}, haystack: {}", 
                                     // needle, haystack); 
                        }
                    }
                }
            }

            // This simulates a preceding '.*?' for every regex by adding
            // a state starting at the current position in the input for the
            // beginning of the program only if we don't already have a match.
            if groups.get(0).is_none() {
                self.add(clist, 0, groups.as_mut_slice())
            }

            // Now we try to read the next character.
            // As a result, the 'step' method will look at the previous
            // character.
            self.ic = next_ic;
            next_ic = self.set_chars(next_ic);

            let mut i = 0;
            while i < clist.size {
                let pc = clist.pc(i);
                match self.step(&mut groups, nlist, clist.groups(i), pc) {
                    StepMatchEarlyReturn => return groups,
                    StepMatch => clist.empty(),
                    StepContinue => {},
                }
                i += 1;
            }
            mem::swap(clist, nlist);
            nlist.empty();
        }
        groups
    }

    fn step(&self, groups: &mut CaptureLocs, nlist: &mut Threads,
            caps: &mut [Option<uint>], pc: uint)
           -> StepState {
        match self.insts[pc] {
            Match => {
                if !self.caps {
                    // This is a terrible hack that is used to
                    // indicate a match when the caller doesn't want
                    // any capture groups.
                    // We can bail out super early since we don't
                    // care about matching leftmost-longest.
                    // return vec!(Some(0), Some(0)) 
                    *groups.get_mut(0) = Some(0);
                    *groups.get_mut(1) = Some(0);
                    return StepMatchEarlyReturn
                } else {
                    let mut i = 0;
                    while i < groups.len() {
                        *groups.get_mut(i) = caps[i];
                        i += 1;
                    }
                    return StepMatch
                }
            }
            OneChar(c, flags) => {
                if self.char_eq(flags & FLAG_NOCASE > 0, self.prev, c) {
                    self.add(nlist, pc+1, caps);
                }
            }
            CharClass(ref ranges, flags) => {
                if self.prev.is_some() {
                    let c = self.prev.unwrap();
                    let negate = flags & FLAG_NEGATED > 0;
                    let casei = flags & FLAG_NOCASE > 0;
                    let found = ranges.as_slice();
                    let found = found.bsearch(|&rc| class_cmp(casei, c, rc));
                    let found = found.is_some();
                    if (found && !negate) || (!found && negate) {
                        self.add(nlist, pc+1, caps);
                    }
                }
            }
            Any(flags) => {
                if flags & FLAG_DOTNL > 0 || !self.char_eq(false, self.prev, '\n') {
                    self.add(nlist, pc+1, caps)
                }
            }
            EmptyBegin(_) | EmptyEnd(_) | EmptyWordBoundary(_)
            | Save(_) | Jump(_) | Split(_, _) => {},
        }
        StepContinue
    }

    fn add(&self, nlist: &mut Threads, pc: uint, groups: &mut [Option<uint>]) {
        if nlist.contains(pc) {
            return
        }
        // We have to add states to the threads list even if their empty.
        // TL;DR - It prevents cycles.
        // If we didn't care about cycles, we'd *only* add threads that
        // correspond to non-jumping instructions (OneChar, Any, Match, etc.).
        // But, it's possible for valid regexs (like '(a*)*') to result in
        // a cycle in the instruction list. e.g., We'll keep chasing the Split
        // instructions forever.
        // So we add these instructions to our thread queue, but in the main
        // VM loop, we look for them but simply ignore them.
        // Adding them to the queue prevents them from being revisited so we
        // can avoid cycles (and the inevitable stack overflow).
        //
        // We make a minor optimization by indicating that the state is "empty"
        // so that its capture groups are not filled in.
        match self.insts[pc] {
            EmptyBegin(flags) => {
                let multi = flags & FLAG_MULTI > 0;
                nlist.add(pc, groups, true);
                if self.is_begin() || (multi && self.char_is(self.prev, '\n')) {
                    self.add(nlist, pc + 1, groups)
                }
            }
            EmptyEnd(flags) => {
                let multi = flags & FLAG_MULTI > 0;
                nlist.add(pc, groups, true);
                if self.is_end() || (multi && self.char_is(self.cur, '\n')) {
                    self.add(nlist, pc + 1, groups)
                }
            }
            EmptyWordBoundary(flags) => {
                nlist.add(pc, groups, true);
                if self.is_word_boundary() == !(flags & FLAG_NEGATED > 0) {
                    self.add(nlist, pc + 1, groups)
                }
            }
            Save(slot) => {
                nlist.add(pc, groups, true);
                if !self.caps {
                    self.add(nlist, pc + 1, groups);
                } else {
                    let old = groups[slot];
                    groups[slot] = Some(self.ic);
                    self.add(nlist, pc + 1, groups);
                    groups[slot] = old;
                }
            }
            Jump(to) => {
                nlist.add(pc, groups, true);
                self.add(nlist, to, groups)
            }
            Split(x, y) => {
                nlist.add(pc, groups, true);
                self.add(nlist, x, groups);
                self.add(nlist, y, groups);
            }
            Match | OneChar(_, _) | CharClass(_, _) | Any(_) => {
                // If captures are enabled, then we need to indicate that
                // this isn't an empty state.
                // Otherwise, we say it's an empty state (even though it isn't)
                // so that capture groups aren't copied.
                nlist.add(pc, groups, !self.caps);
            }
        }
    }

    fn is_begin(&self) -> bool { self.prev.is_none() }
    fn is_end(&self) -> bool {
        // println!("ic: {}, prev: {}, cur: {}, next: {}", 
                 // self.ic, self.prev, self.cur, self.next); 
        self.cur.is_none()
    }

    fn is_word_boundary(&self) -> bool {
        if self.is_begin() {
            return self.is_word(self.cur)
        }
        if self.is_end() {
            return self.is_word(self.prev)
        }
        (self.is_word(self.cur) && !self.is_word(self.prev))
        || (self.is_word(self.prev) && !self.is_word(self.cur))
    }

    fn is_word(&self, c: Option<char>) -> bool {
        let c = match c { None => return false, Some(c) => c };
        c == '_'
        || (c >= '0' && c <= '9')
        || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
    }

    // FIXME: For case insensitive comparisons, it uses the uppercase
    // character and tests for equality. IIUC, this does not generalize to
    // all of Unicode. I believe we need to check the entire fold for each
    // character. This will be easy to add if and when it gets added to Rust's
    // standard library.
    #[inline(always)]
    fn char_eq(&self, casei: bool, textc: Option<char>, regc: char) -> bool {
        match textc {
            None => false,
            Some(textc) => {
                regc == textc
                    || (casei && regc.to_uppercase() == textc.to_uppercase())
            }
        }
    }

    #[inline(always)]
    fn char_is(&self, textc: Option<char>, regc: char) -> bool {
        textc == Some(regc)
    }

    fn set_chars(&mut self, ic: uint) -> uint {
        self.prev = None;
        self.cur = None;
        self.next = None;
        if self.input.len() == 0 {
            return 0 + 1
        }
        if ic > 0 {
            let i = cmp::min(ic, self.input.len());
            let prev = self.input.char_range_at_reverse(i);
            self.prev = Some(prev.ch);
        }
        if ic < self.input.len() {
            let cur = self.input.char_range_at(ic);
            self.cur = Some(cur.ch);
            if ic + cur.next < self.input.len() {
                let next = self.input.char_range_at(cur.next);
                self.next = Some(next.ch);
            }
            cur.next
        } else {
            self.input.len() + 1
        }
    }
}

struct Thread {
    pc: uint,
    groups: CaptureLocs,
}

impl Thread {
    fn new(pc: uint, groups: CaptureLocs) -> Thread {
        Thread { pc: pc, groups: groups }
    }
}

struct Threads {
    queue: Vec<Thread>,
    sparse: Vec<uint>,
    size: uint,
}

impl Threads {
    // This is using a wicked neat trick to provide constant time lookup
    // for threads in the queue using a sparse set. A queue of threads is
    // allocated once with maximal size when the VM initializes and is reused
    // throughout execution. That is, there should be zero allocation during
    // the execution of a VM.
    //
    // See http://research.swtch.com/sparse for the deets.
    fn new(num_insts: uint, num_caps: uint) -> Threads {
        Threads {
            queue: Vec::from_fn(num_insts, |_| {
                Thread::new(0, Vec::from_elem(num_caps * 2, None))
            }),
            sparse: Vec::from_elem(num_insts, 0u),
            size: 0,
        }
    }

    fn add(&mut self, pc: uint, groups: &[Option<uint>], empty: bool) {
        let t = self.queue.get_mut(self.size);
        t.pc = pc;
        if !empty {
            let mut i = 0;
            while i < groups.len() {
                *t.groups.get_mut(i) = groups[i];
                i += 1;
            }
        }
        *self.sparse.get_mut(pc) = self.size;
        self.size += 1;
    }

    #[inline(always)]
    fn contains(&self, pc: uint) -> bool {
        let s = *self.sparse.get(pc);
        s < self.size && self.queue.get(s).pc == pc
    }

    fn empty(&mut self) {
        self.size = 0;
    }

    fn pc(&self, i: uint) -> uint {
        self.queue.get(i).pc
    }

    fn groups<'r>(&'r mut self, i: uint) -> &'r mut [Option<uint>] {
        self.queue.get_mut(i).groups.as_mut_slice()
    }
}

/// Given a character and a single character class range, return an ordering
/// indicating whether the character is less than the start of the range,
/// in the range (inclusive) or greater than the end of the range.
///
/// If `casei` is `true`, then this ordering is computed case insensitively.
///
/// This function is meant to be used with a binary search.
#[inline(always)]
fn class_cmp(casei: bool, mut textc: char,
             (mut start, mut end): (char, char)) -> Ordering {
    if casei {
        // FIXME: This is pretty ridiculous. All of this case conversion
        // can be moved outside this function:
        // 1) textc should be uppercased outside the bsearch.
        // 2) the character class itself should be uppercased either in the
        //    parser or the compiler.
        // FIXME: This is too simplistic for correct Unicode support.
        //        See also: char_eq
        textc = textc.to_uppercase();
        start = start.to_uppercase();
        end = end.to_uppercase();
    }
    if textc >= start && textc <= end {
        Equal
    } else if start > textc {
        Greater
    } else {
        Less
    }
}

/// Returns the starting location of `needle` in `haystack`.
/// If `needle` is not in `haystack`, then `None` is returned.
///
/// Note that this is using a naive substring algorithm.
fn find_prefix(needle: &[u8], haystack: &[u8]) -> Option<uint> {
    if needle.len() > haystack.len() || needle.len() == 0 {
        return None
    }
    let mut hayi = 0u;
    'HAYSTACK: loop {
        if hayi > haystack.len() - needle.len() {
            break
        }
        let mut nedi = 0u;
        while nedi < needle.len() {
            if haystack[hayi+nedi] != needle[nedi] {
                hayi += 1;
                continue 'HAYSTACK
            }
            nedi += 1;
        }
        return Some(hayi)
    }
    None
}
