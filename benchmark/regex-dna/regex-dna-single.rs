// Originally written by JustAPerson (https://github.com/JustAPerson).
// Modified by Andrew Gallant (https://github.com/BurntSushi).

#![feature(macro_rules, phase)]

extern crate regex;
#[phase(syntax)]extern crate regex_macros;

use regex::{NoExpand, Regex};

fn replace(re: &Regex, text: &str, rep: &str) -> ~str {
    re.replace_all(text, NoExpand(rep))
}

fn count_matches(seq: &str, variant: &Regex) -> int {
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

    seq = regex!(">[^\n]*\n|\n").replace_all(seq, NoExpand(""));
    let clen = seq.len();

    let variants = ~[
        regex!("agggtaaa|tttaccct"),
        regex!("[cgt]gggtaaa|tttaccc[acg]"),
        regex!("a[act]ggtaaa|tttacc[agt]t"),
        regex!("ag[act]gtaaa|tttac[agt]ct"),
        regex!("agg[act]taaa|ttta[agt]cct"),
        regex!("aggg[acg]aaa|ttt[cgt]ccct"),
        regex!("agggt[cgt]aa|tt[acg]accct"),
        regex!("agggta[cgt]a|t[acg]taccct"),
        regex!("agggtaa[cgt]|[acg]ttaccct"),
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
        (regex!("B"), "(c|g|t)"),
        (regex!("D"), "(a|g|t)"),
        (regex!("H"), "(a|c|t)"),
        (regex!("K"), "(g|t)"),
        (regex!("M"), "(a|c)"),
        (regex!("N"), "(a|c|g|t)"),
        (regex!("R"), "(a|g)"),
        (regex!("S"), "(c|g)"),
        (regex!("V"), "(a|c|g)"),
        (regex!("W"), "(a|t)"),
        (regex!("Y"), "(c|t)"),
    ];
    for (re, replacement) in substs.move_iter() {
        seq = replace(&re, seq, replacement)
    }
    println!("");
    println!("{}", ilen);
    println!("{}", clen);
    println!("{}", seq.len());
}
