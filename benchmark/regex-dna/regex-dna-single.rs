// Originally written by JustAPerson (https://github.com/JustAPerson).
// Modified by Andrew Gallant (https://github.com/BurntSushi).

#![feature(macro_rules, phase)]

extern crate regexp;
#[phase(syntax)]extern crate regexp_macros;

use regexp::{NoExpand, Regexp};
 
fn replace(re: &Regexp, text: &str, rep: &str) -> ~str {
    re.replace_all(text, NoExpand(rep))
}
 
fn count_matches(seq: &str, variant: &Regexp) -> int {
    let mut n = 0;
    for _ in variant.find_iter(seq) {
        n += 1;
    }
    n
}
 
fn main() {
    let mut stdin =  std::io::stdio::stdin();
    let mut seq = stdin.read_to_str().unwrap();
    let ilen = seq.len();

    seq = regexp!(">[^\n]*\n|\n").replace_all(seq, NoExpand(""));
    let clen = seq.len();

    let variants = ~[
        regexp!("agggtaaa|tttaccct"),
        regexp!("[cgt]gggtaaa|tttaccc[acg]"),
        regexp!("a[act]ggtaaa|tttacc[agt]t"),
        regexp!("ag[act]gtaaa|tttac[agt]ct"),
        regexp!("agg[act]taaa|ttta[agt]cct"),
        regexp!("aggg[acg]aaa|ttt[cgt]ccct"),
        regexp!("agggt[cgt]aa|tt[acg]accct"),
        regexp!("agggta[cgt]a|t[acg]taccct"),
        regexp!("agggtaa[cgt]|[acg]ttaccct"),
    ];
    let (mut variant_strs, mut counts) = (vec!(), vec!());
    for variant in variants.move_iter() {
        variant_strs.push(variant.to_str().to_owned());
        counts.push(count_matches(seq, &variant));
    }
    for (i, variant) in variant_strs.iter().enumerate() {
        println!("{} {}", variant, *counts.get(i));
    }
 
    let substs = ~[
        (regexp!("B"), "(c|g|t)"),
        (regexp!("D"), "(a|g|t)"),
        (regexp!("H"), "(a|c|t)"),
        (regexp!("K"), "(g|t)"),
        (regexp!("M"), "(a|c)"),
        (regexp!("N"), "(a|c|g|t)"),
        (regexp!("R"), "(a|g)"),
        (regexp!("S"), "(c|g)"),
        (regexp!("V"), "(a|c|g)"),
        (regexp!("W"), "(a|t)"),
        (regexp!("Y"), "(c|t)"),
    ];
    for (re, replacement) in substs.move_iter() {
        seq = replace(&re, seq, replacement)
    }
    println!("");
    println!("{}", ilen);
    println!("{}", clen);
    println!("{}", seq.len());
}
