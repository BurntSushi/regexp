#![allow(unused_imports)]

use rand::{Rng, task_rng};
use stdtest::BenchHarness;
use std::str;
use super::super::{Regexp, NoExpand};

fn bench_assert_match(b: &mut BenchHarness, re: Regexp, text: &str) {
    b.iter(|| if !re.is_match(text) { fail!("no match") });
}

#[bench]
fn no_exponential(b: &mut BenchHarness) {
    let n = 100;
    let re = Regexp::new("a?".repeat(n) + "a".repeat(n)).unwrap();
    let text = "a".repeat(n);
    bench_assert_match(b, re, text);
}

#[bench]
fn literal(b: &mut BenchHarness) {
    let re = Regexp::new("y").unwrap();
    let text = "x".repeat(50) + "y";
    bench_assert_match(b, re, text);
}

#[bench]
fn not_literal(b: &mut BenchHarness) {
    let re = Regexp::new(".y").unwrap();
    let text = "x".repeat(50) + "y";
    bench_assert_match(b, re, text);
}

#[bench]
fn match_class(b: &mut BenchHarness) {
    let re = Regexp::new("[abcdw]").unwrap();
    let text = "xxxx".repeat(20) + "w";
    bench_assert_match(b, re, text);
}

#[bench]
fn match_class_in_range(b: &mut BenchHarness) {
    // 'b' is between 'a' and 'c', so the class range checking doesn't help.
    let re = Regexp::new("[ac]").unwrap();
    let text = "bbbb".repeat(20) + "c";
    bench_assert_match(b, re, text);
}

#[bench]
fn replace_all(b: &mut BenchHarness) {
    let re = Regexp::new("[cjrw]").unwrap();
	let text = "abcdefghijklmnopqrstuvwxyz";
    // FIXME: This isn't using the $name expand stuff.
    // It's possible RE2/Go is using it, but currently, the expand in this
    // crate is actually compiling a regex, so it's incredibly slow.
    b.iter(|| re.replace_all(text, NoExpand("")));
}

#[bench]
fn anchored_literal_short_non_match(b: &mut BenchHarness) {
    let re = Regexp::new("^zbc(d|e)").unwrap();
    let text = "abcdefghijklmnopqrstuvwxyz";
    b.iter(|| re.is_match(text));
}

#[bench]
fn anchored_literal_long_non_match(b: &mut BenchHarness) {
    let re = Regexp::new("^zbc(d|e)").unwrap();
    let text = "abcdefghijklmnopqrstuvwxyz".repeat(15);
    b.iter(|| re.is_match(text));
}

#[bench]
fn anchored_literal_short_match(b: &mut BenchHarness) {
    let re = Regexp::new("^.bc(d|e)").unwrap();
    let text = "abcdefghijklmnopqrstuvwxyz";
    b.iter(|| re.is_match(text));
}

#[bench]
fn anchored_literal_long_match(b: &mut BenchHarness) {
    let re = Regexp::new("^.bc(d|e)").unwrap();
    let text = "abcdefghijklmnopqrstuvwxyz".repeat(15);
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_short_a(b: &mut BenchHarness) {
    let re = Regexp::new("^.bc(d|e)*$").unwrap();
    let text = "abcddddddeeeededd";
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_short_a_not(b: &mut BenchHarness) {
    let re = Regexp::new(".bc(d|e)*$").unwrap();
    let text = "abcddddddeeeededd";
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_short_b(b: &mut BenchHarness) {
    let re = Regexp::new("^.bc(?:d|e)*$").unwrap();
    let text = "abcddddddeeeededd";
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_short_b_not(b: &mut BenchHarness) {
    let re = Regexp::new(".bc(?:d|e)*$").unwrap();
    let text = "abcddddddeeeededd";
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_long_prefix(b: &mut BenchHarness) {
    let re = Regexp::new("^abcdefghijklmnopqrstuvwxyz.*$").unwrap();
    let text = "abcdefghijklmnopqrstuvwxyz";
    b.iter(|| re.is_match(text));
}

#[bench]
fn one_pass_long_prefix_not(b: &mut BenchHarness) {
    let re = Regexp::new("^.bcdefghijklmnopqrstuvwxyz.*$").unwrap();
    let text = "abcdefghijklmnopqrstuvwxyz";
    b.iter(|| re.is_match(text));
}

macro_rules! throughput(
    ($name:ident, $regex:ident, $size:expr) => (
        #[bench]
        fn $name(b: &mut BenchHarness) {
            let re = Regexp::new($regex).unwrap();
            let text = gen_text($size);
            b.bytes = $size;
            b.iter(|| if re.is_match(text) { fail!("match") });
        }
    );
)

static EASY0: &'static str  = "ABCDEFGHIJKLMNOPQRSTUVWXYZ$";
static EASY1: &'static str  = "A[AB]B[BC]C[CD]D[DE]E[EF]F[FG]G[GH]H[HI]I[IJ]J$";
static MEDIUM: &'static str = "[XYZ]ABCDEFGHIJKLMNOPQRSTUVWXYZ$";
static HARD: &'static str   = "[ -~]*ABCDEFGHIJKLMNOPQRSTUVWXYZ$";

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

// The 32MB tests take too damn long.
// I actually think this is the fault of the microbenchmark facilities built
// into rustc. Go's microbenchmarking seems to handle things fine.

throughput!(easy0_32, EASY0, 32)
throughput!(easy0_1K, EASY0, 1<<10)
throughput!(easy0_32K, EASY0, 32<<10)
throughput!(easy0_1M, EASY0, 1<<20)
// throughput!(easy0_32M, EASY0, 32<<20) 

throughput!(easy1_32, EASY1, 32)
throughput!(easy1_1K, EASY1, 1<<10)
throughput!(easy1_32K, EASY1, 32<<10)
throughput!(easy1_1M, EASY1, 1<<20)
// throughput!(easy1_32M, EASY1, 32<<20) 

throughput!(medium_32, MEDIUM, 32)
throughput!(medium_1K, MEDIUM, 1<<10)
throughput!(medium_32K, MEDIUM, 32<<10)
throughput!(medium_1M, MEDIUM, 1<<20)
// throughput!(medium_32M, MEDIUM, 32<<20) 

throughput!(hard_32, HARD, 32)
throughput!(hard_1K, HARD, 1<<10)
throughput!(hard_32K, HARD, 32<<10)
throughput!(hard_1M, HARD, 1<<20)
// throughput!(hard_32M, HARD, 32<<20) 

