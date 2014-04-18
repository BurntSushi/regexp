// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use rand::{Rng, task_rng};
use stdtest::Bencher;
use std::str;
use regexp::{Regexp, NoExpand};

fn bench_assert_match(b: &mut Bencher, re: Regexp, text: &str) {
    b.iter(|| if !re.is_match(text) { fail!("no match") });
}

#[bench]
fn no_exponential(b: &mut Bencher) {
    let n = 100;
    let re = Regexp::new("a?".repeat(n) + "a".repeat(n)).unwrap();
    let text = "a".repeat(n);
    bench_assert_match(b, re, text);
}

#[bench]
fn literal(b: &mut Bencher) {
    let re = regexp!("y");
    let text = "x".repeat(50) + "y";
    bench_assert_match(b, re, text);
}

#[bench]
fn not_literal(b: &mut Bencher) {
    let re = regexp!(".y");
    let text = "x".repeat(50) + "y";
    bench_assert_match(b, re, text);
}

#[bench]
fn match_class(b: &mut Bencher) {
    let re = regexp!("[abcdw]");
    let text = "xxxx".repeat(20) + "w";
    bench_assert_match(b, re, text);
}

#[bench]
fn match_class_in_range(b: &mut Bencher) {
    // 'b' is between 'a' and 'c', so the class range checking doesn't help.
    let re = regexp!("[ac]");
    let text = "bbbb".repeat(20) + "c";
    bench_assert_match(b, re, text);
}

#[bench]
fn replace_all(b: &mut Bencher) {
    let re = regexp!("[cjrw]");
    let text = "abcdefghijklmnopqrstuvwxyz";
    // FIXME: This isn't using the $name expand stuff.
    // It's possible RE2/Go is using it, but currently, the expand in this
    // crate is actually compiling a regex, so it's incredibly slow.
    b.iter(|| re.replace_all(text, NoExpand("")));
}

#[bench]
fn anchored_literal_short_non_match(b: &mut Bencher) {
    let re = regexp!("^zbc(d|e)");
    let text = "abcdefghijklmnopqrstuvwxyz";
    b.iter(|| re.is_match(text));
}

#[bench]
fn anchored_literal_long_non_match(b: &mut Bencher) {
    let re = regexp!("^zbc(d|e)");
    let text = "abcdefghijklmnopqrstuvwxyz".repeat(15);
    b.iter(|| re.is_match(text));
}

#[bench]
fn anchored_literal_short_match(b: &mut Bencher) {
    let re = regexp!("^.bc(d|e)");
    let text = "abcdefghijklmnopqrstuvwxyz";
    b.iter(|| re.is_match(text));
}

#[bench]
fn anchored_literal_long_match(b: &mut Bencher) {
    let re = regexp!("^.bc(d|e)");
    let text = "abcdefghijklmnopqrstuvwxyz".repeat(15);
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_short_a(b: &mut Bencher) {
    let re = regexp!("^.bc(d|e)*$");
    let text = "abcddddddeeeededd";
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_short_a_not(b: &mut Bencher) {
    let re = regexp!(".bc(d|e)*$");
    let text = "abcddddddeeeededd";
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_short_b(b: &mut Bencher) {
    let re = regexp!("^.bc(?:d|e)*$");
    let text = "abcddddddeeeededd";
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_short_b_not(b: &mut Bencher) {
    let re = regexp!(".bc(?:d|e)*$");
    let text = "abcddddddeeeededd";
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_long_prefix(b: &mut Bencher) {
    let re = regexp!("^abcdefghijklmnopqrstuvwxyz.*$");
    let text = "abcdefghijklmnopqrstuvwxyz";
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_long_prefix_not(b: &mut Bencher) {
    let re = regexp!("^.bcdefghijklmnopqrstuvwxyz.*$");
    let text = "abcdefghijklmnopqrstuvwxyz";
    b.iter(|| re.is_match(text));
}

macro_rules! throughput(
    ($name:ident, $regex:expr, $size:expr) => (
        #[bench]
        fn $name(b: &mut Bencher) {
            let re = regexp!($regex);
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

throughput!(easy0_32, "ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32)
throughput!(easy0_1K, "ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<10)
throughput!(easy0_32K, "ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32<<10)
// throughput!(easy0_1M, "ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<20) 
// throughput!(easy0_32M, EASY0, 32<<20) 

throughput!(easy1_32, "A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$", 32)
throughput!(easy1_1K, "A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$", 1<<10)
throughput!(easy1_32K, "A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$", 32<<10)
// throughput!(easy1_1M, "A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$", 1<<20) 
// throughput!(easy1_32M, EASY1, 32<<20) 

throughput!(medium_32, "[XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32)
throughput!(medium_1K, "[XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<10)
throughput!(medium_32K, "[XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32<<10)
// throughput!(medium_1M, "[XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<20) 
// throughput!(medium_32M, MEDIUM, 32<<20) 

throughput!(hard_32, "[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32)
throughput!(hard_1K, "[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<10)
throughput!(hard_32K, "[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 32<<10)
// throughput!(hard_1M, "[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$", 1<<20) 
// throughput!(hard_32M, HARD, 32<<20) 

