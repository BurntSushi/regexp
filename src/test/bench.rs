// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(dead_code, unused_imports)]

use rand::{Rng, task_rng};
use stdtest::Bencher;
use std::str;
use regexp::{Regexp, Dynamic, NoExpand};
use regexp::{MatchKind, Exists, Location, Submatches};

// fn bench_assert_match<R: Regexp>(b: &mut Bencher, re: R, text: &str) { 
    // b.iter(|| if !re.is_match(text) { fail!("no match") }); 
// } 
//  
// #[bench] 
// fn no_exponential(b: &mut Bencher) { 
    // let n = 100; 
    // let re = Dynamic::new("a?".repeat(n) + "a".repeat(n)).unwrap(); 
    // let text = "a".repeat(n); 
    // bench_assert_match(b, re, text); 
// } 
//  
// #[bench] 
// fn literal(b: &mut Bencher) { 
    // let re = nregexp!("y"); 
    // let text = "x".repeat(50) + "y"; 
    // bench_assert_match(b, re, text); 
// } 
//  
// #[bench] 
// fn not_literal(b: &mut Bencher) { 
    // let re = nregexp!(".y"); 
    // let text = "x".repeat(50) + "y"; 
    // bench_assert_match(b, re, text); 
// } 
//  
// #[bench] 
// fn match_class(b: &mut Bencher) { 
    // let re = nregexp!("[abcdw]"); 
    // let text = "xxxx".repeat(20) + "w"; 
    // bench_assert_match(b, re, text); 
// } 
//  
// #[bench] 
// fn match_class_in_range(b: &mut Bencher) { 
    // // 'b' is between 'a' and 'c', so the class range checking doesn't help. 
    // let re = nregexp!("[ac]"); 
    // let text = "bbbb".repeat(20) + "c"; 
    // bench_assert_match(b, re, text); 
// } 
//  
// #[bench] 
// fn replace_all(b: &mut Bencher) { 
    // let re = nregexp!("[cjrw]"); 
    // let text = "abcdefghijklmnopqrstuvwxyz"; 
    // // FIXME: This isn't using the $name expand stuff. 
    // // It's possible RE2/Go is using it, but currently, the expand in this 
    // // crate is actually compiling a regex, so it's incredibly slow. 
    // b.iter(|| re.replace_all(text, NoExpand(""))); 
// } 
//  
// #[bench] 
// fn anchored_literal_short_non_match(b: &mut Bencher) { 
    // let re = nregexp!("^zbc(d|e)"); 
    // let text = "abcdefghijklmnopqrstuvwxyz"; 
    // b.iter(|| re.is_match(text)); 
// } 
//  
// #[bench] 
// fn anchored_literal_long_non_match(b: &mut Bencher) { 
    // let re = nregexp!("^zbc(d|e)"); 
    // let text = "abcdefghijklmnopqrstuvwxyz".repeat(15); 
    // b.iter(|| re.is_match(text)); 
// } 
//  
// #[bench] 
// fn anchored_literal_short_match(b: &mut Bencher) { 
    // let re = nregexp!("^.bc(d|e)"); 
    // let text = "abcdefghijklmnopqrstuvwxyz"; 
    // b.iter(|| re.is_match(text)); 
// } 
//  
// #[bench] 
// fn anchored_literal_long_match(b: &mut Bencher) { 
    // let re = nregexp!("^.bc(d|e)"); 
    // let text = "abcdefghijklmnopqrstuvwxyz".repeat(15); 
    // b.iter(|| re.is_match(text)); 
// } 
//  
// #[bench] 
// fn one_pass_short_a(b: &mut Bencher) { 
    // let re = nregexp!("^.bc(d|e)*$"); 
    // let text = "abcddddddeeeededd"; 
    // b.iter(|| re.is_match(text)); 
// } 
//  
// #[bench] 
// fn one_pass_short_a_not(b: &mut Bencher) { 
    // let re = nregexp!(".bc(d|e)*$"); 
    // let text = "abcddddddeeeededd"; 
    // b.iter(|| re.is_match(text)); 
// } 
//  
// #[bench] 
// fn one_pass_short_b(b: &mut Bencher) { 
    // let re = nregexp!("^.bc(?:d|e)*$"); 
    // let text = "abcddddddeeeededd"; 
    // b.iter(|| re.is_match(text)); 
// } 
//  
// #[bench] 
// fn one_pass_short_b_not(b: &mut Bencher) { 
    // let re = nregexp!(".bc(?:d|e)*$"); 
    // let text = "abcddddddeeeededd"; 
    // b.iter(|| re.is_match(text)); 
// } 
//  
// #[bench] 
// fn one_pass_long_prefix(b: &mut Bencher) { 
    // let re = nregexp!("^abcdefghijklmnopqrstuvwxyz.*$"); 
    // let text = "abcdefghijklmnopqrstuvwxyz"; 
    // b.iter(|| re.is_match(text)); 
// } 
//  
// #[bench] 
// fn one_pass_long_prefix_not(b: &mut Bencher) { 
    // let re = nregexp!("^.bcdefghijklmnopqrstuvwxyz.*$"); 
    // let text = "abcdefghijklmnopqrstuvwxyz"; 
    // b.iter(|| re.is_match(text)); 
// } 

macro_rules! throughput(
    ($name:ident, $regex:expr, $size:expr) => (
        #[bench]
        fn $name(b: &mut Bencher) {
            let re = nregexp!($regex);
            let text = gen_text($size);
            b.bytes = $size;
            b.iter(|| if re.is_match(text) { fail!("match") });
        }
    );
)

// static EASY0: &'static str  = "ABCDEFGHIJKLMNOPQRSTUVWXYZ$"; 
// static EASY1: &'static str  = "A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$"; 
// static MEDIUM: &'static str = "[XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$"; 
// static HARD: &'static str   = "[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$"; 

fn gen_text(n: uint) -> ~str {
    let mut rng = task_rng();
    let mut bytes = rng.gen_ascii_str(n).into_bytes();
    for (i, b) in bytes.mut_iter().enumerate() {
        if i % 20 == 0 {
            *b = '\n' as u8
        }
    }
    str::from_utf8(bytes).unwrap().to_owned()
}

// The 1MB/32MB benchmarks take too damn long and they typically aren't
// substantially different from the 32K benchmarks.
// I actually think this is the fault of the microbenchmark facilities built
// into rustc. Go's microbenchmarking seems to handle things fine.

// throughput!(easy0_32, "ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32) 
// throughput!(easy0_1K, "ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<10) 
// throughput!(easy0_32K, "ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32<<10) 
// throughput!(easy0_1M, "ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<20) 
// throughput!(easy0_32M, EASY0, 32<<20) 

// throughput!(easy1_32, "A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$", 32) 
// throughput!(easy1_1K, "A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$", 1<<10) 
// throughput!(easy1_32K, "A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$", 32<<10) 
// throughput!(easy1_1M, "A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$", 1<<20) 
// throughput!(easy1_32M, EASY1, 32<<20) 

// throughput!(medium_32, "[XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32) 
// throughput!(medium_1K, "[XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<10) 
// throughput!(medium_32K, "[XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32<<10) 
// throughput!(medium_1M, "[XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<20) 
// throughput!(medium_32M, MEDIUM, 32<<20) 

// throughput!(hard_32, "[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32) 
// throughput!(hard_1K, "[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<10) 
// throughput!(hard_32K, "[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32<<10) 
// throughput!(hard_1M, "[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<20) 
// throughput!(hard_32M, HARD, 32<<20) 

// type Captures = ~[Option<uint>]; 
struct Nfa<'t> {
    which: MatchKind,
    input: &'t str,
    ic: uint,
    chars: CharReader<'t>,
}
enum StepState { StepMatchEarlyReturn, StepMatch, StepContinue, }
impl <'t> Nfa<'t> {
    fn run(&mut self, start: uint, end: uint) -> ~[Option<uint>] {
        let mut matched = false;
        // let clist = &mut Threads::new(self.which); 
        // let nlist = &mut Threads::new(self.which); 
        let clist = unsafe { &mut Threads{which: self.which,
                queue: ::std::mem::uninit(),
                sparse: ::std::mem::uninit(),
                size: 0,} };
        let nlist = unsafe { &mut Threads{which: self.which,
                queue: ::std::mem::uninit(),
                sparse: ::std::mem::uninit(),
                size: 0,} };
        let groups = &mut [None, None];
        let prefix_anchor = false;
        self.ic = start;
        let mut next_ic = self.chars.set(start);
        while self.ic <= end {
            if clist.size == 0 { if matched { break  } { } }
            if clist.size == 0 || (!prefix_anchor && !matched) {
                self.add(clist, 0, groups)
            }
            self.ic = next_ic;
            next_ic = self.chars.advance();
            let mut i = 0;
            while i < clist.size {
                let pc = clist.pc(i);
                let step_state =
                    self.step(groups, nlist, &mut [], pc);
                match step_state {
                    StepMatchEarlyReturn =>
                    return ~[Some(0u), Some(0u)],
                    StepMatch => {
                        matched = true;
                        clist.empty()
                    },
                    StepContinue => { }
                }
                i += 1;
            }
            ::std::mem::swap(clist, nlist);
            nlist.empty();
        }
        match self.which {
            Exists if matched => ~[Some(0u), Some(0u)],
            Exists => ~[None, None],
            Location | Submatches => groups.into_owned()
        }
    }
    #[allow(unused_variable)]
    fn step(&self, groups: &mut [Option<uint>], nlist: &mut Threads,
            caps: &mut [Option<uint>], pc: uint) -> StepState {
        match pc {
            0u => { },
            1u => { },
            2u => {
                if self.chars.prev.is_some() {
                    let c = self.chars.prev.unwrap();
                    let found = [(' ', '~')];
                    let found =
                        found.bsearch(|&rc|
                                          class_cmp(false, c,
                                                    rc));
                    let found = found.is_some();
                    if found { self.add(nlist, 3u, caps); }
                }
            },
            3u => { },
            4u => {
                if self.chars.prev == Some('A') {
                    self.add(nlist, 5u, caps);
                }
            },
            5u => {
                if self.chars.prev == Some('B') {
                    self.add(nlist, 6u, caps);
                }
            },
            6u => {
                if self.chars.prev == Some('C') {
                    self.add(nlist, 7u, caps);
                }
            },
            7u => {
                if self.chars.prev == Some('D') {
                    self.add(nlist, 8u, caps);
                }
            },
            8u => {
                if self.chars.prev == Some('E') {
                    self.add(nlist, 9u, caps);
                }
            },
            9u => {
                if self.chars.prev == Some('F') {
                    self.add(nlist, 10u, caps);
                }
            },
            10u => {
                if self.chars.prev == Some('G') {
                    self.add(nlist, 11u, caps);
                }
            },
            11u => {
                if self.chars.prev == Some('H') {
                    self.add(nlist, 12u, caps);
                }
            },
            12u => {
                if self.chars.prev == Some('I') {
                    self.add(nlist, 13u, caps);
                }
            },
            13u => {
                if self.chars.prev == Some('J') {
                    self.add(nlist, 14u, caps);
                }
            },
            14u => {
                if self.chars.prev == Some('K') {
                    self.add(nlist, 15u, caps);
                }
            },
            15u => {
                if self.chars.prev == Some('L') {
                    self.add(nlist, 16u, caps);
                }
            },
            16u => {
                if self.chars.prev == Some('M') {
                    self.add(nlist, 17u, caps);
                }
            },
            17u => {
                if self.chars.prev == Some('N') {
                    self.add(nlist, 18u, caps);
                }
            },
            18u => {
                if self.chars.prev == Some('O') {
                    self.add(nlist, 19u, caps);
                }
            },
            19u => {
                if self.chars.prev == Some('P') {
                    self.add(nlist, 20u, caps);
                }
            },
            20u => {
                if self.chars.prev == Some('Q') {
                    self.add(nlist, 21u, caps);
                }
            },
            21u => {
                if self.chars.prev == Some('R') {
                    self.add(nlist, 22u, caps);
                }
            },
            22u => {
                if self.chars.prev == Some('S') {
                    self.add(nlist, 23u, caps);
                }
            },
            23u => {
                if self.chars.prev == Some('T') {
                    self.add(nlist, 24u, caps);
                }
            },
            24u => {
                if self.chars.prev == Some('U') {
                    self.add(nlist, 25u, caps);
                }
            },
            25u => {
                if self.chars.prev == Some('V') {
                    self.add(nlist, 26u, caps);
                }
            },
            26u => {
                if self.chars.prev == Some('W') {
                    self.add(nlist, 27u, caps);
                }
            },
            27u => {
                if self.chars.prev == Some('X') {
                    self.add(nlist, 28u, caps);
                }
            },
            28u => {
                if self.chars.prev == Some('Y') {
                    self.add(nlist, 29u, caps);
                }
            },
            29u => {
                if self.chars.prev == Some('Z') {
                    self.add(nlist, 30u, caps);
                }
            },
            30u => { },
            31u => { },
            32u => {
                match self.which {
                    Exists => { return StepMatchEarlyReturn },
                    Location => {
                        groups[0] = caps[0];
                        groups[1] = caps[1];
                        return StepMatch
                    },
                    Submatches => {
                        unsafe {
                            groups.copy_memory(caps.as_slice())
                        }
                        return StepMatch
                    }
                }
            },
            _ => { }
        }
        StepContinue
    }
    fn add(&self, nlist: &mut Threads, pc: uint,
           groups: &mut [Option<uint>]) {
        if nlist.contains(pc) { return }
        match pc {
            0u => {
                nlist.add(0u, groups, true);
                match self.which {
                    Submatches | Location => {
                        let old = groups[0u];
                        groups[0u] = Some(self.ic);
                        self.add(nlist, 1u, groups);
                        groups[0u] = old;
                    },
                    Exists => self.add(nlist, 1u, groups)
                }
            },
            1u => {
                nlist.add(1u, groups, true);
                self.add(nlist, 2u, groups);
                self.add(nlist, 4u, groups);
            },
            2u => nlist.add(2u, groups, false),
            3u => {
                nlist.add(3u, groups, true);
                self.add(nlist, 1u, groups);
            },
            4u => nlist.add(4u, groups, false),
            5u => nlist.add(5u, groups, false),
            6u => nlist.add(6u, groups, false),
            7u => nlist.add(7u, groups, false),
            8u => nlist.add(8u, groups, false),
            9u => nlist.add(9u, groups, false),
            10u => nlist.add(10u, groups, false),
            11u => nlist.add(11u, groups, false),
            12u => nlist.add(12u, groups, false),
            13u => nlist.add(13u, groups, false),
            14u => nlist.add(14u, groups, false),
            15u => nlist.add(15u, groups, false),
            16u => nlist.add(16u, groups, false),
            17u => nlist.add(17u, groups, false),
            18u => nlist.add(18u, groups, false),
            19u => nlist.add(19u, groups, false),
            20u => nlist.add(20u, groups, false),
            21u => nlist.add(21u, groups, false),
            22u => nlist.add(22u, groups, false),
            23u => nlist.add(23u, groups, false),
            24u => nlist.add(24u, groups, false),
            25u => nlist.add(25u, groups, false),
            26u => nlist.add(26u, groups, false),
            27u => nlist.add(27u, groups, false),
            28u => nlist.add(28u, groups, false),
            29u => nlist.add(29u, groups, false),
            30u => {
                nlist.add(30u, groups, true);
                if self.is_end() { self.add(nlist, 31u, groups) }
            },
            31u => {
                nlist.add(31u, groups, true);
                match self.which {
                    Submatches | Location => {
                        let old = groups[1u];
                        groups[1u] = Some(self.ic);
                        self.add(nlist, 32u, groups);
                        groups[1u] = old;
                    },
                    Exists => self.add(nlist, 32u, groups)
                }
            },
            32u => nlist.add(32u, groups, false),
            _ => { }
        }
    }
    #[allow(dead_code)]
    fn is_begin(&self) -> bool { self.chars.prev.is_none() }
    #[allow(dead_code)]
    fn is_end(&self) -> bool { self.chars.cur.is_none() }
    #[allow(dead_code)]
    fn is_word_boundary(&self) -> bool {
        if self.is_begin() { return self.is_word(self.chars.cur) }
        if self.is_end() { return self.is_word(self.chars.prev) }
        (self.is_word(self.chars.cur) &&
             !self.is_word(self.chars.prev)) ||
            (self.is_word(self.chars.prev) &&
                 !self.is_word(self.chars.cur))
    }
    #[allow(dead_code)]
    fn is_word(&self, c: Option<char>) -> bool {
        let c = match c { None => return false, Some(c) => c };
        c == '_' || (c >= '0' && c <= '9') ||
            (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')
    }
}
struct CharReader<'t> {
    input: &'t str,
    prev: Option<char>,
    cur: Option<char>,
    next: uint,
}
impl <'t> CharReader<'t> {
    fn set(&mut self, ic: uint) -> uint {
        self.prev = None;
        self.cur = None;
        self.next = 0;
        if self.input.len() == 0 { return 0 + 1 }
        if ic > 0 {
            let i = ::std::cmp::min(ic, self.input.len());
            let prev = self.input.char_range_at_reverse(i);
            self.prev = Some(prev.ch);
        }
        if ic < self.input.len() {
            let cur = self.input.char_range_at(ic);
            self.cur = Some(cur.ch);
            self.next = cur.next;
            self.next
        } else { self.input.len() + 1 }
    }
    fn advance(&mut self) -> uint {
        self.prev = self.cur;
        if self.next < self.input.len() {
            let cur = self.input.char_range_at(self.next);
            self.cur = Some(cur.ch);
            self.next = cur.next;
        } else {
            self.cur = None;
            self.next = self.input.len() + 1;
        }
        self.next
    }
}
struct Threads {
    which: MatchKind,
    queue: [uint, ..33],
    sparse: [uint, ..33],
    size: uint,
}
impl Threads {
    fn new(which: MatchKind) -> Threads {
        Threads{which: which,
                queue:
                    [0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                     0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                     0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u],
                sparse:
                    [0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                     0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u,
                     0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u, 0u],
                size: 0,}
    }
    fn add(&mut self, pc: uint, groups: &[Option<uint>], empty: bool) {
        self.queue[self.size] = pc;
        self.sparse[pc] = self.size;
        self.size += 1;
    }
    fn contains(&self, pc: uint) -> bool {
        let s = self.sparse[pc];
        s < self.size && self.queue[s] == pc
    }
    fn empty(&mut self) { self.size = 0; }
    fn pc(&self, i: uint) -> uint { self.queue[i] }
}
fn class_cmp(casei: bool, mut textc: char,
             (mut start, mut end): (char, char)) -> Ordering {
    if casei {
        textc = textc.to_uppercase();
        start = start.to_uppercase();
        end = end.to_uppercase();
    }
    if textc >= start && textc <= end {
        Equal
    } else if start > textc { Greater } else { Less }
}


fn exec<'t>(which: ::regexp::MatchKind, input: &'t str,
            start: uint, end: uint) -> ~[Option<uint>] {
    return Nfa{which: which,
               input: input,
               ic: 0,
               chars:
                   CharReader{input: input,
                              prev: None,
                              cur: None,
                              next: 0,},}.run(start, end);
}

#[bench]
fn hard_1MB(b: &mut Bencher) {
    // let re = nregexp!("[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$"); 
    let size = (1<<20) as uint;
    let text = gen_text(size);
    b.bytes = size as u64;
    b.iter(|| exec(::regexp::Exists, text, 0, text.len()));
    // b.iter(|| if re.is_match(text) { fail!("match") }); 
}

