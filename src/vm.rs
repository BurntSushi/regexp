use std::iter;
use super::compile::{Inst, Char, Match, Jump, Split};

fn run(insts: Vec<Inst>, input: &str) -> bool {
    Vm {
        insts: insts,
        input: input.chars().collect(),
        ready: vec!(),
    }.run()
}

#[deriving(Show)]
struct Thread {
    pc: uint,
}

#[deriving(Show)]
struct Threads {
    threads: Vec<uint>,
}

impl Threads {
    fn new() -> Threads {
        Threads { threads: vec!() }
    }

    fn add(&mut self, pc: uint) {
        self.threads.push(pc);
    }

    fn empty(&mut self) {
        self.threads = vec!()
    }

    fn swap(&mut self, other: Threads) {
        self.threads = other.threads;
    }

    fn len(&self) -> uint {
        self.threads.len()
    }

    fn get(&self, i: uint) -> uint {
        *self.threads.get(i)
    }
}

struct Vm {
    insts: Vec<Inst>,
    input: Vec<char>,
    ready: Vec<Thread>,
}

impl Vm {
    fn run(&mut self) -> bool {
        let mut matched = false;
        let (mut clist, mut nlist) = (Threads::new(), Threads::new());
        clist.add(0);
        for ic in iter::range_inclusive(0, self.input.len()) {
            let mut i = 0;
            while i < clist.len() {
                let pc = clist.get(i);
                match *self.insts.get(pc) {
                    Char(c) => {
                        if ic < self.input.len() && c == *self.input.get(ic) {
                            self.add(&mut nlist, pc + 1);
                        }
                    }
                    Match => {
                        matched = true;
                        clist.empty();
                    }
                    Jump(_) => unreachable!(),
                    Split(_, _) => unreachable!(),
                }
                i += 1;
            }

            clist.swap(nlist);
            nlist = Threads::new();
        }
        matched
    }

    fn add(&mut self, threads: &mut Threads, pc: uint) {
        match *self.insts.get(pc) {
            Jump(to) => self.add(threads, to),
            Split(x, y) => {
                self.add(threads, x);
                self.add(threads, y);
            }
            _ => threads.add(pc),
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::parse;
    use super::super::compile;

    #[test]
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
