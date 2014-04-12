// Originally written by JustAPerson (https://github.com/JustAPerson).
// Modified by Andrew Gallant (https://github.com/BurntSushi).

extern crate regexp;
extern crate sync;

use regexp::{Regexp, NoExpand};
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
    for _ in re.find_iter(seq) {
        n += 1;
    }
    n
}
 
fn main() {
    let mut stdin =  std::io::stdio::stdin();
    let mut seq = stdin.read_to_str().unwrap();
    let ilen = seq.len();
 
    seq = Regexp::new(">[^\n]*\n|\n").unwrap().replace_all(seq, NoExpand(""));
    let seq_arc = Arc::new(seq.clone());
    let clen = seq.len();
 
    let mut counts = vec!();
    for &variant in VARIANTS.iter() {
        let seq_arc_copy = seq_arc.clone();
        let count = sync::Future::spawn(proc() {
            count_matches(*seq_arc_copy, variant)
        });
        counts.push(count);
    }
 
    let mut seqlen = sync::Future::spawn(proc() {
        let mut seq = seq;
        for &Subst(k, v) in SUBST.iter() {
            seq = replace(seq, k, v);
        }
        seq.len()
    });
    for (i, variant) in VARIANTS.iter().enumerate() {
        println!("{} {:?}", variant, counts.get_mut(i).get());
    }
    println!("");
    println!("{}", ilen);
    println!("{}", clen);
    println!("{}", seqlen.get());
}
