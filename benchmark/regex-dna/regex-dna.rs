// Originally written by JustAPerson (https://github.com/JustAPerson).
// Modified by Andrew Gallant (https://github.com/BurntSushi).

#![feature(macro_rules, phase)]

extern crate regexp;
#[phase(syntax)]extern crate regexp_macros_exp;
extern crate sync;

use regexp::{NoExpand, Regexp};
use sync::Arc;

static VARIANTS: &'static [&'static str] = &'static [
    "agggtaaa|tttaccct",
    "[cgt]gggtaaa|tttaccc[acg]",
    "a[act]ggtaaa|tttacc[agt]t",
    "ag[act]gtaaa|tttac[agt]ct",
    "agg[act]taaa|ttta[agt]cct",
    "aggg[acg]aaa|ttt[cgt]ccct",
    "agggt[cgt]aa|tt[acg]accct",
    "agggta[cgt]a|t[acg]taccct",
    "agggtaa[cgt]|[acg]ttaccct",
];
 
struct Subst<'a>(~Regexp, &'a str);
 
fn replace<R: Regexp>(re: R, text: &str, rep: &str) -> ~str {
    re.replace_all(text, NoExpand(rep))
}
 
fn count_matches<R: Regexp>(seq: &str, variant: R) -> int {
    let mut n = 0;
    for _ in variant.find_iter(seq) {
        n += 1;
    }
    n
}

macro_rules! future_variant(
    ($seq_arc:ident, $regex:expr) => ({
        let seq_arc_copy = $seq_arc.clone();
        sync::Future::spawn(proc() {
            let re = nregexp!($regex);
            count_matches(*seq_arc_copy, re)
        })
    });
)
 
fn main() {
    let mut stdin =  std::io::stdio::stdin();
    let mut seq = stdin.read_to_str().unwrap();
    let ilen = seq.len();

    let trim_headers = nregexp!(">[^\n]*\n|\n");
    seq = replace(trim_headers, seq, "");
    let seq_arc = Arc::new(seq.clone());
    let clen = seq.len();
 
    let mut counts = vec!();
    counts.push(future_variant!(seq_arc, "agggtaaa|tttaccct"));
    counts.push(future_variant!(seq_arc, "[cgt]gggtaaa|tttaccc[acg]"));
    counts.push(future_variant!(seq_arc, "a[act]ggtaaa|tttacc[agt]t"));
    counts.push(future_variant!(seq_arc, "ag[act]gtaaa|tttac[agt]ct"));
    counts.push(future_variant!(seq_arc, "agg[act]taaa|ttta[agt]cct"));
    counts.push(future_variant!(seq_arc, "aggg[acg]aaa|ttt[cgt]ccct"));
    counts.push(future_variant!(seq_arc, "agggt[cgt]aa|tt[acg]accct"));
    counts.push(future_variant!(seq_arc, "agggta[cgt]a|t[acg]taccct"));
    counts.push(future_variant!(seq_arc, "agggtaa[cgt]|[acg]ttaccct"));
 
    let mut seqlen = sync::Future::spawn(proc() {
        let substs = ~[
            Subst(~nregexp!("B") as ~Regexp, "(c|g|t)"),
            Subst(~nregexp!("D") as ~Regexp, "(a|g|t)"),
            Subst(~nregexp!("H") as ~Regexp, "(a|c|t)"),
            Subst(~nregexp!("K") as ~Regexp, "(g|t)"),
            Subst(~nregexp!("M") as ~Regexp, "(a|c)"),
            Subst(~nregexp!("N") as ~Regexp, "(a|c|g|t)"),
            Subst(~nregexp!("R") as ~Regexp, "(a|g)"),
            Subst(~nregexp!("S") as ~Regexp, "(c|g)"),
            Subst(~nregexp!("V") as ~Regexp, "(a|c|g)"),
            Subst(~nregexp!("W") as ~Regexp, "(a|t)"),
            Subst(~nregexp!("Y") as ~Regexp, "(c|t)"),
        ];
        let mut seq = seq;
        for Subst(re, replacement) in substs.move_iter() {
            seq = replace(re, seq, replacement)
        }
        seq.len()
    });
    for (i, variant) in VARIANTS.iter().enumerate() {
        println!("{} {}", variant, counts.get_mut(i).get());
    }
    println!("");
    println!("{}", ilen);
    println!("{}", clen);
    println!("{}", seqlen.get());
}
