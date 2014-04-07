use std::cmp;
use std::iter;
use std::mem;
use super::compile::{Inst, Char, CharClass, Any,
                     EmptyBegin, EmptyEnd, EmptyWordBoundary,
                     Match, Save, Jump, Split};

pub type CaptureIndices = Vec<Option<(uint, uint)>>;

pub fn run(insts: &[Inst], input: &[char]) -> CaptureIndices {
    unflatten_capture_locations(Vm {
        insts: insts,
        input: input,
    }.run())
}

fn unflatten_capture_locations(locs: Vec<Option<uint>>) -> CaptureIndices {
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

struct Thread {
    pc: uint,
    groups: Vec<Option<uint>>,
}

impl Thread {
    fn new(pc: uint, groups: Vec<Option<uint>>) -> Thread {
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
    // throughout execution.
    //
    // See http://research.swtch.com/sparse for the deets.
    fn new(num_insts: uint, num_caps: uint) -> Threads {
        Threads {
            queue: Vec::from_fn(num_insts, |_| {
                Thread::new(0, Vec::from_elem(num_caps, None))
            }),
            sparse: Vec::from_elem(num_insts, 0u),
            size: 0,
        }
    }

    fn add(&mut self, pc: uint, groups: &[Option<uint>]) {
        assert!(pc < self.sparse.len());
        if !self.contains(pc) {
            let t = self.queue.get_mut(self.size);
            t.pc = pc;
            for (i, &v) in groups.iter().enumerate() {
                *t.groups.get_mut(i) = v
            }
            *self.sparse.get_mut(pc) = self.size;
            self.size += 1;
        }
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

struct Vm<'r> {
    insts: &'r [Inst],
    input: &'r [char],
}

impl<'r> Vm<'r> {
    fn run(&self) -> Vec<Option<uint>> {
        let num_caps = numcaps(self.insts);
        let mut clist = Threads::new(self.insts.len(), num_caps);
        let mut nlist = Threads::new(self.insts.len(), num_caps);

        let mut groups = Vec::from_elem(num_caps, None);
        self.add(&mut clist, 0, 0, groups.as_mut_slice());

        for ic in iter::range_inclusive(0, self.input.len()) {
            if clist.size == 0 && nlist.size == 0 {
                break
            }
            let mut i = 0;
            while i < clist.size {
                let pc = clist.pc(i);
                match self.insts[pc] {
                    Match => {
                        groups = Vec::from_slice(clist.groups(i));
                        clist.empty();
                    }
                    Char(c, casei) => {
                        if self.char_eq(casei, ic, c) {
                            self.add(&mut nlist, pc+1, ic+1, clist.groups(i));
                        }
                    }
                    CharClass(ref ranges, negate, casei) => {
                        if ic < self.input.len() {
                            let c = self.get(ic);
                            let found = ranges.as_slice();
                            let found = found.bsearch(|&rc| class_cmp(casei, c, rc));
                            let found = found.is_some();
                            if (found && !negate) || (!found && negate) {
                                self.add(&mut nlist, pc+1, ic+1, clist.groups(i));
                            }
                        }
                    }
                    Any(true) =>
                        self.add(&mut nlist, pc+1, ic+1, clist.groups(i)),
                    Any(false) => {
                        if !self.char_eq(false, ic, '\n') {
                            self.add(&mut nlist, pc+1, ic+1, clist.groups(i))
                        }
                    }
                    // These cases are handled in 'add'
                    EmptyBegin(_) => {},
                    EmptyEnd(_) => {},
                    EmptyWordBoundary(_) => {},
                    Save(_) => {},
                    Jump(_) => {},
                    Split(_, _) => {},
                }
                i += 1;
            }
            mem::swap(&mut clist, &mut nlist);
            nlist.empty();
        }
        groups
    }

    fn add(&self, nlist: &mut Threads, pc: uint, ic: uint,
           groups: &mut [Option<uint>]) {
        if nlist.contains(pc) {
            return
        }
        // This is absolutely critical to the *correctness* of the VM.
        // TL;DR - It prevents cycles.
        // If we didn't care about cycles, we'd *only* add threads that
        // correspond to non-jumping instructions (Char, Any, Match, etc.).
        // But, it's possible for valid regexs (like '(a*)*') to result in
        // a cycle in the instruction list. e.g., We'll keep chasing the Split
        // instructions forever.
        // So we add these instructions to our thread queue, but in the main
        // VM loop, we look for them but simply ignore them.
        // Adding them to the queue prevents them from being revisited so we
        // can avoid cycles (and the inevitable stack overflow).
        nlist.add(pc, groups);
        match self.insts[pc] {
            EmptyBegin(multi) => {
                if self.is_begin(ic) || (multi && self.char_is(ic-1, '\n')) {
                    self.add(nlist, pc + 1, ic, groups)
                }
            }
            EmptyEnd(multi) => {
                if self.is_end(ic) || (multi && self.char_is(ic, '\n')) {
                    self.add(nlist, pc + 1, ic, groups)
                }
            }
            EmptyWordBoundary(yes) => {
                let wb = self.is_word_boundary(ic);
                if yes == wb {
                    self.add(nlist, pc + 1, ic, groups)
                }
            }
            Save(slot) => {
                let old = groups[slot];
                groups[slot] = Some(ic);
                self.add(nlist, pc + 1, ic, groups);
                groups[slot] = old;
            }
            Jump(to) => self.add(nlist, to, ic, groups),
            Split(x, y) => {
                self.add(nlist, x, ic, groups);
                self.add(nlist, y, ic, groups);
            }
            // Handled in 'run'
            Match | Char(_, _) | CharClass(_, _, _) | Any(_) => {},
        }
    }

    fn is_begin(&self, ic: uint) -> bool { ic == 0 }
    fn is_end(&self, ic: uint) -> bool { ic == self.input.len() }

    fn is_word_boundary(&self, ic: uint) -> bool {
        if self.is_begin(ic) {
            return self.is_word(ic)
        }
        if self.is_end(ic) {
            return self.is_word(self.input.len()-1)
        }
        (self.is_word(ic) && !self.is_word(ic-1))
        || (self.is_word(ic-1) && !self.is_word(ic))
    }

    fn is_word(&self, ic: uint) -> bool {
        if ic >= self.input.len() {
            return false
        }
        let c = self.input[ic];
        c == '_'
        || (c >= '0' && c <= '9')
        || (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
    }

    // FIXME: For case insensitive comparisons, it uses the uppercase
    // character and tests for equality. IIUC, this does not generalize to
    // all of Unicode. I believe we need to check the entire fold for each
    // character. This will be easy to add if and when it gets added to Rust's
    // standard library.
    fn char_eq(&self, casei: bool, ic: uint, regc: char) -> bool {
        if ic >= self.input.len() {
            return false
        }
        let textc = self.get(ic);
        regc == textc || (casei && regc.to_uppercase() == textc.to_uppercase())
    }

    fn char_is(&self, ic: uint, c: char) -> bool {
        ic < self.input.len() && self.input[ic] == c
    }

    fn get(&self, ic: uint) -> char {
        self.input[ic]
    }
}

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

fn numcaps(insts: &[Inst]) -> uint {
    let mut n = 0;
    for inst in insts.iter() {
        match *inst {
            Save(c) => n = cmp::max(n, c+1),
            _ => {}
        }
    }
    n
}
