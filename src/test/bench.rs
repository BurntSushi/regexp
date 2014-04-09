#![allow(dead_code)]

use rand::{Rng, task_rng};
use stdtest::BenchHarness;
use std::str;
use super::super::Regexp;

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

throughput!(easy0_32, EASY0, 32)
throughput!(easy0_1K, EASY0, 1<<10)
throughput!(easy0_32K, EASY0, 32<<10)
// throughput!(easy0_1M, EASY0, 1<<20)
// throughput!(easy0_32M, EASY0, 32<<20)

throughput!(easy1_32, EASY1, 32)
throughput!(easy1_1K, EASY1, 1<<10)
throughput!(easy1_32K, EASY1, 32<<10)
// throughput!(easy1_1M, EASY1, 1<<20)
// throughput!(easy1_32M, EASY1, 32<<20)

throughput!(medium_32, MEDIUM, 32)
throughput!(medium_1K, MEDIUM, 1<<10)
throughput!(medium_32K, MEDIUM, 32<<10)
// throughput!(medium_1M, MEDIUM, 1<<20)
// throughput!(medium_32M, MEDIUM, 32<<20)

throughput!(hard_32, HARD, 32)
throughput!(hard_1K, HARD, 1<<10)
throughput!(hard_32K, HARD, 32<<10)
throughput!(hard_64K, HARD, 64<<10)
// throughput!(hard_1M, HARD, 1<<20) 
// throughput!(hard_32M, HARD, 32<<20) 

