use std::cmp;
use std::iter;
use std::mem;
use super::compile::{Inst, Char, Any, EmptyBegin, EmptyEnd, Match, Save, Jump, Split};

pub fn run(insts: Vec<Inst>, input: &str) -> Option<uint> {
    Vm {
        insts: insts,
        input: input.chars().collect(),
    }.run()
}

fn numcaps(insts: &[Inst]) -> uint {
    let mut n = 0;
    for &inst in insts.iter() {
        match inst {
            Save(c) => n = cmp::max(n, c+1),
            _ => {}
        }
    }
    n
}

#[deriving(Show)]
struct Thread {
    pc: uint,
    groups: Vec<uint>,
}

impl Thread {
    fn new(pc: uint, groups: &[uint]) -> Thread {
        // Dunno if this conditional is needed. Perhaps the from_slice is
        // automatically optimized away when len(groups) == 0.
        // FIXME: Benchmark this.
        if groups.len() == 0 {
            Thread { pc: pc, groups: vec!(), }
        } else {
            Thread { pc: pc, groups: Vec::from_slice(groups) }
        }
    }
}

#[deriving(Show)]
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
    fn new(num_insts: uint) -> Threads {
        Threads {
            queue: Vec::from_fn(num_insts, |_| Thread::new(0, [])),
            sparse: Vec::from_elem(num_insts, 0u),
            size: 0,
        }
    }

    fn add(&mut self, pc: uint, groups: &[uint]) {
        assert!(pc < self.sparse.len());
        if !self.contains(pc) {
            *self.sparse.get_mut(pc) = self.size;
            *self.queue.get_mut(self.size) = Thread::new(pc, groups);
            self.size += 1;
        }
    }

    fn contains(&self, pc: uint) -> bool {
        assert!(pc < self.sparse.len());
        let s = *self.sparse.get(pc);
        s < self.size && self.queue.get(s).pc == pc
    }

    fn empty(&mut self) {
        self.size = 0;
    }

    fn pc(&self, i: uint) -> uint {
        self.queue.get(i).pc
    }

    fn groups<'r>(&'r mut self, i: uint) -> &'r mut [uint] {
        self.queue.get_mut(i).groups.as_mut_slice()
    }

    fn save(&mut self, i: uint, slot: uint, ic: uint) {
        *self.queue.get_mut(i).groups.get_mut(slot) = ic
    }
}

struct Vm {
    insts: Vec<Inst>,
    input: Vec<char>,
}

impl Vm {
    fn run(&mut self) -> Option<uint> {
        let mut matched = None;
        let mut clist = Threads::new(self.insts.len());
        let mut nlist = Threads::new(self.insts.len());
        let mut groups = Vec::from_elem(numcaps(self.insts.as_slice()), 0u);
        self.add(&mut clist, 0, 0, groups.as_mut_slice());

        for ic in iter::range_inclusive(0, self.input.len()) {
            let mut i = 0;
            while i < clist.size {
                let pc = clist.pc(i);
                match *self.insts.get(pc) {
                    Match => {
                        matched = Some(ic);
                        groups = Vec::from_slice(clist.groups(i));
                        clist.empty();
                    }
                    Char(c, casei) => {
                        if self.char_eq(casei, ic, c) {
                            self.add(&mut nlist, pc + 1, ic + 1, clist.groups(i));
                        }
                    }
                    Any(true) => self.add(&mut nlist, pc + 1, ic + 1, clist.groups(i)),
                    Any(false) => {
                        if !self.char_eq(false, ic, '\n') {
                            self.add(&mut nlist, pc + 1, ic + 1, clist.groups(i))
                        }
                    }
                    // These cases are handled in 'add'
                    EmptyBegin(_) => {},
                    EmptyEnd(_) => {},
                    Save(_) => {},
                    Jump(_) => {},
                    Split(_, _) => {},
                }
                i += 1;
            }
            mem::swap(&mut clist, &mut nlist);
            nlist.empty();
        }
        debug!("GROUPS: {}", groups);
        matched
    }

    fn add(&mut self, threads: &mut Threads,
           pc: uint, ic: uint, groups: &mut [uint]) {
        if threads.contains(pc) {
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
        threads.add(pc, groups);
        match *self.insts.get(pc) {
            EmptyBegin(multi) => {
                if ic == 0 || (multi && self.char_is(ic-1, '\n')) {
                    self.add(threads, pc + 1, ic, groups)
                }
            }
            EmptyEnd(multi) => {
                if ic == self.input.len()
                   || (multi && self.char_is(ic, '\n')) {
                    self.add(threads, pc + 1, ic, groups)
                }
            }
            Save(slot) => {
                debug!("SAVING {} in slot {}", ic, slot);
                // clist.save(i, slot, ic); 
                // let groups = clist.groups(i); 
                let old = groups[slot];
                groups[slot] = ic;
                self.add(threads, pc + 1, ic, groups);
                groups[slot] = old;
            }
            Jump(to) => self.add(threads, to, ic, groups),
            Split(x, y) => {
                self.add(threads, x, ic, groups);
                self.add(threads, y, ic, groups);
            }
            // Handled in 'run'
            Match | Char(_, _) | Any(_) => {},
        }
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
        ic < self.input.len() && *self.input.get(ic) == c
    }

    fn get(&self, ic: uint) -> char {
        *self.input.get(ic)
    }
}

#[cfg(test)]
mod test {
    use super::super::parse;
    use super::super::compile;

    #[test]
    #[ignore]
    fn simple() {
        let re = "a+b+?";
        let re = match parse::parse(re) {
            Err(err) => fail!("Parse error: {}", err),
            Ok(re) => re,
        };
        // debug!("RE: {}", re); 
        let insts = compile::compile(re);
        debug!("Insts: {}", insts);
        debug!("{}", super::run(insts, "abbbbbbbc"));
    }
}
