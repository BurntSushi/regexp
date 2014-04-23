// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// ignore-stage1
// ignore-cross-compile #12102

#![feature(macro_rules, phase)]

extern crate regexp;
#[phase(syntax)]extern crate regexp_macros;
extern crate sync;

use regexp::{NoExpand, Regexp};
use sync::Arc;

fn count_matches(seq: &str, variant: &Regexp) -> int {
    let mut n = 0;
    for _ in variant.find_iter(seq) {
        n += 1;
    }
    n
}

fn main() {
    let mut seq = if std::os::getenv("RUST_BENCH").is_some() {
        let fd = std::io::File::open(&Path::new("shootout-k-nucleotide.data"));
        std::io::BufferedReader::new(fd).read_to_str().unwrap()
    } else {
        std::io::stdin().read_to_str().unwrap()
    };
    let ilen = seq.len();

    seq = regexp!(">[^\n]*\n|\n").replace_all(seq, NoExpand(""));
    let seq_arc = Arc::new(seq.clone()); // copy before it moves
    let clen = seq.len();

    let mut seqlen = sync::Future::spawn(proc() {
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
        let mut seq = seq;
        for (re, replacement) in substs.move_iter() {
            seq = re.replace_all(seq, NoExpand(replacement));
        }
        seq.len()
    });

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
        let seq_arc_copy = seq_arc.clone();
        variant_strs.push(variant.to_str().to_owned());
        counts.push(sync::Future::spawn(proc() {
            count_matches(*seq_arc_copy, &variant)
        }));
    }

    for (i, variant) in variant_strs.iter().enumerate() {
        println!("{} {}", variant, counts.get_mut(i).get());
    }
    println!("");
    println!("{}", ilen);
    println!("{}", clen);
    println!("{}", seqlen.get());
}
