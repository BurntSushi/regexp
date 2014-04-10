// Originally written by JustAPerson (https://github.com/JustAPerson).
// Modified by Andrew Gallant (https://github.com/BurntSushi).

#![feature(phase)]

#[phase(syntax, link)] extern crate regexp;

use regexp::{Regexp, NoExpand};
 
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
 
struct Subst<'a>(&'a str, &'a str);

static SUBST: &'static [Subst<'static>] = &'static [
    Subst("B", "(c|g|t)"), Subst("D", "(a|g|t)"),   Subst("H", "(a|c|t)"),
    Subst("K", "(g|t)"),   Subst("M", "(a|c)"),     Subst("N", "(a|c|g|t)"),
    Subst("R", "(a|g)"),   Subst("S", "(c|g)"),     Subst("V", "(a|c|g)"),
    Subst("W", "(a|t)"),   Subst("Y", "(c|t)"),
];
 
fn replace(text: &str, regex: &str, rep: &str) -> ~str {
    let re = Regexp::new(regex).unwrap();
    re.replace_all(text, NoExpand(rep))
}
 
fn count_matches(seq: &str, variant: &str) -> int {
    let re = Regexp::new(variant).unwrap();
    let mut n = 0;
    for _ in re.captures_iter(seq) {
        n += 1;
    }
    return n
}
 
fn main() {
    let mut stdin =  std::io::stdio::stdin();
    let mut seq = stdin.read_to_str().unwrap();
    let ilen = seq.len();
 
    seq = re!("(>[^\n]+)?\n").replace_all(seq, NoExpand(""));
    let clen = seq.len();
 
    for variant in VARIANTS.iter() {
        println!("{} {}", variant, count_matches(seq,*variant));
    }
    println!("");
 
    for &Subst(k, v) in SUBST.iter() {
        seq = replace(seq, k, v);
    }
    println!("{}", ilen);
    println!("{}", clen);
    println!("{}", seq.len());
}
