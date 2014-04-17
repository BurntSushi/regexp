// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_id = "regexp_macros_exp#0.11-pre"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]
#![license = "MIT/ASL2"]
#![doc(html_logo_url = "http://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "http://www.rust-lang.org/favicon.ico",
       html_root_url = "http://static.rust-lang.org/doc/master")]

#![feature(macro_registrar, managed_boxes, quote)]

#![allow(unused_imports, unused_variable, dead_code)]

//! This crate provides the `regexp!` macro. Its use is documented in the 
//! `regexp` crate.

extern crate regexp;
extern crate syntax;

use syntax::ast;
use syntax::ast::{Name, TokenTree, TTTok, DUMMY_NODE_ID};
use syntax::ast::{Expr, Expr_, ExprLit, LitStr, ExprVec};
use syntax::codemap::{Span, DUMMY_SP};
use syntax::ext::base::{
    SyntaxExtension, ExtCtxt, MacResult, MRItem, MRExpr, MRAny, AnyMacro,
    NormalTT, BasicMacroExpander,
};
use syntax::parse;
use syntax::parse::token;
use syntax::parse::token::{EOF, LIT_CHAR, IDENT, COMMA};

use syntax::print::pprust;

use regexp::Dynamic;
use regexp::program::{
    MaybeStatic, Flags,
    Inst, OneChar, CharClass, Any, Save, Jump, Split,
    Match, EmptyBegin, EmptyEnd, EmptyWordBoundary,
};

static FLAG_EMPTY:      u8 = 0;
static FLAG_NOCASE:     u8 = 1 << 0; // i
static FLAG_MULTI:      u8 = 1 << 1; // m
static FLAG_DOTNL:      u8 = 1 << 2; // s
static FLAG_SWAP_GREED: u8 = 1 << 3; // U
static FLAG_NEGATED:    u8 = 1 << 4; // char class or not word boundary

/// For the `regexp!` syntax extension. Do not use.
#[macro_registrar]
pub fn macro_registrar(reg: |Name, SyntaxExtension|) {
    reg(token::intern("nregexp"),
        NormalTT(~BasicMacroExpander {
            expander: re_static,
            span: None,
        },
        None));
}

fn re_static(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> MacResult {
    let regex = match parse(cx, tts) {
        Some(r) => r,
        None => return MacResult::dummy_expr(sp),
    };
    let re = match Dynamic::new(regex.to_owned()) {
        Ok(re) => re,
        Err(err) => {
            cx.span_err(sp, err.to_str());
            return MacResult::dummy_expr(sp)
        }
    };

    let (under, zero, nine, a, z, ca, cz) = ('_', '0', '9', 'a', 'z', 'A', 'Z');
    let num_cap_locs = 2 * re.p.num_captures();
    let num_insts = re.p.insts.len();
    let cap_names = as_expr_vec(cx, sp, re.p.names.as_slice(),
        |cx, _, name| match name {
            &Some(ref name) => {
                let name = name.as_slice();
                quote_expr!(cx, Some(::std::str::Owned(~$name)))
            }
            &None => quote_expr!(cx, None),
        }
    );
    let prefix_anchor = 
        match re.p.insts.as_slice()[1] {
            EmptyBegin(flags) if flags & FLAG_MULTI == 0 => true,
            _ => false,
        };
    let init_groups = vec_from_fn(cx, sp, num_cap_locs,
                                  |cx| quote_expr!(&*cx, None));
    let check_prefix = mk_check_prefix(cx, sp, &re);
    let step_insts = mk_step_insts(cx, sp, &re);
    let add_insts = mk_add_insts(cx, sp, &re);
    let expr = quote_expr!(&*cx, {
        struct RegexpNative {
            cap_names: ~[Option<::std::str::MaybeOwned<'static>>],
        }
        impl ::regexp::Regexp for RegexpNative {
            fn capture_names<'r>(&'r self) -> &'r [Option<::std::str::MaybeOwned<'static>>] {
                self.cap_names.as_slice()
            }

            fn exec<'t>(&self, which: ::regexp::MatchKind, input: &'t str,
                        start: uint, end: uint) -> ~[Option<uint>] {
                use regexp::{MatchKind, Exists, Location, Submatches};

                return Nfa {
                    which: which,
                    input: input,
                    ic: 0,
                    chars: CharReader {
                        input: input,
                        prev: None,
                        cur: None,
                        next: 0,
                    },
                }.run(start, end);

                type Captures = [Option<uint>, ..$num_cap_locs];

                struct Nfa<'t> {
                    which: MatchKind,
                    input: &'t str,
                    ic: uint,
                    chars: CharReader<'t>,
                }

                enum StepState {
                    StepMatchEarlyReturn,
                    StepMatch,
                    StepContinue,
                }

                impl<'t> Nfa<'t> {
                    fn run(&mut self, start: uint, end: uint) -> ~[Option<uint>] {
                        let mut matched = false;
                        let mut clist = &mut Threads::new(self.which);
                        let mut nlist = &mut Threads::new(self.which);

                        let mut groups = $init_groups;

                        self.ic = start;
                        let mut next_ic = self.chars.set(start);
                        while self.ic <= end {
                            if clist.size == 0 {
                                if matched {
                                    break
                                }
                                $check_prefix
                            }
                            if clist.size == 0 || (!$prefix_anchor && !matched) {
                                self.add(clist, 0, &mut groups)
                            }

                            self.ic = next_ic;
                            next_ic = self.chars.advance();

                            let mut i = 0;
                            while i < clist.size {
                                let pc = clist.pc(i);
                                let step_state = self.step(&mut groups, nlist,
                                                           clist.groups(i), pc);
                                match step_state {
                                    StepMatchEarlyReturn => return [Some(0u), Some(0u)].into_owned(),
                                    StepMatch => { matched = true; clist.empty() },
                                    StepContinue => {},
                                }
                                i += 1;
                            }
                            ::std::mem::swap(&mut clist, &mut nlist);
                            nlist.empty();
                        }
                        match self.which {
                            Exists if matched     => ~[Some(0u), Some(0u)],
                            Exists                => ~[None, None],
                            Location | Submatches => groups.into_owned(),
                        }
                    }

                    // Sometimes `nlist` is never used (for empty regexes).
                    #[allow(unused_variable)]
                    fn step(&self, groups: &mut Captures, nlist: &mut Threads,
                            caps: &mut Captures, pc: uint)
                           -> StepState {
                        $step_insts
                        StepContinue
                    }

                    fn add(&self, nlist: &mut Threads, pc: uint,
                           groups: &mut Captures) {
                        if nlist.contains(pc) {
                            return
                        }
                        $add_insts
                    }

                    #[allow(dead_code)]
                    fn is_begin(&self) -> bool { self.chars.prev.is_none() }
                    #[allow(dead_code)]
                    fn is_end(&self) -> bool { self.chars.cur.is_none() }

                    #[allow(dead_code)]
                    fn is_word_boundary(&self) -> bool {
                        if self.is_begin() {
                            return self.is_word(self.chars.cur)
                        }
                        if self.is_end() {
                            return self.is_word(self.chars.prev)
                        }
                        (self.is_word(self.chars.cur) && !self.is_word(self.chars.prev))
                        || (self.is_word(self.chars.prev) && !self.is_word(self.chars.cur))
                    }

                    #[allow(dead_code)]
                    fn is_word(&self, c: Option<char>) -> bool {
                        let c = match c { None => return false, Some(c) => c };
                        c == $under
                        || (c >= $zero && c <= $nine)
                        || (c >= $a && c <= $z) || (c >= $ca && c <= $cz)
                    }
                }

                struct CharReader<'t> {
                    input: &'t str,
                    prev: Option<char>,
                    cur: Option<char>,
                    next: uint,
                }

                impl<'t> CharReader<'t> {
                    fn set(&mut self, ic: uint) -> uint {
                        self.prev = None;
                        self.cur = None;
                        self.next = 0;

                        if self.input.len() == 0 {
                            return 0 + 1
                        }
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
                        } else {
                            self.input.len() + 1
                        }
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

                struct Thread {
                    pc: uint,
                    groups: Captures,
                }

                struct Threads {
                    which: MatchKind,
                    queue: [Thread, ..$num_insts],
                    sparse: [uint, ..$num_insts],
                    size: uint,
                }

                impl Threads {
                    fn new(which: MatchKind) -> Threads {
                        Threads {
                            which: which,
                            queue: unsafe { ::std::mem::uninit() },
                            sparse: unsafe { ::std::mem::uninit() },
                            size: 0,
                        }
                    }

                    fn add(&mut self, pc: uint, groups: &Captures, empty: bool) {
                        let t = &mut self.queue[self.size];
                        t.pc = pc;
                        match (empty, self.which) {
                            (_, Exists) | (true, _) => {},
                            (false, Location) => {
                                t.groups[0] = groups[0];
                                t.groups[1] = groups[1];
                            }
                            (false, Submatches) => {
                                unsafe { t.groups.copy_memory(groups.as_slice()) }
                            }
                        }
                        self.sparse[pc] = self.size;
                        self.size += 1;
                    }

                    #[inline(always)]
                    fn contains(&self, pc: uint) -> bool {
                        let s = self.sparse[pc];
                        s < self.size && self.queue[s].pc == pc
                    }

                    fn empty(&mut self) {
                        self.size = 0;
                    }

                    fn pc(&self, i: uint) -> uint {
                        self.queue[i].pc
                    }

                    fn groups<'r>(&'r mut self, i: uint) -> &'r mut Captures {
                        &'r mut self.queue[i].groups
                    }
                }

                #[allow(dead_code)]
                #[inline(always)]
                fn class_cmp(casei: bool, mut textc: char,
                             (mut start, mut end): (char, char)) -> Ordering {
                    if casei {
                        textc = textc.to_uppercase();
                        start = start.to_uppercase();
                        end = end.to_uppercase();
                    }
                    if textc >= start && textc <= end {
                        Equal
                    } else if start > textc {
                        Greater
                    } else {
                        Less
                    }
                }

                #[allow(dead_code)]
                fn find_prefix(needle: &[u8], haystack: &[u8]) -> Option<uint> {
                    if needle.len() > haystack.len() || needle.len() == 0 {
                        return None
                    }
                    let mut hayi = 0u;
                    'HAYSTACK: loop {
                        if hayi > haystack.len() - needle.len() {
                            break
                        }
                        for nedi in ::std::iter::range(0, needle.len()) {
                            if haystack[hayi+nedi] != needle[nedi] {
                                hayi += 1;
                                continue 'HAYSTACK
                            }
                        }
                        return Some(hayi)
                    }
                    None
                }
            }
        }
        RegexpNative { cap_names: ~$cap_names }
    });
    // println!("{}", pprust::expr_to_str(expr)); 
    MRExpr(expr)
}

// This trait is defined in the quote module in the syntax crate, but I
// don't think it's exported.
// Interestingly, quote_expr! only requires that a 'to_tokens' method be
// defined rather than satisfying a particular trait.
// I think these should be included in the `syntax` crate anyway.
#[doc(hidden)]
trait ToTokens {
    fn to_tokens(&self, cx: &ExtCtxt) -> Vec<TokenTree>;
}

impl ToTokens for char {
    fn to_tokens(&self, _: &ExtCtxt) -> Vec<TokenTree> {
        vec!(TTTok(DUMMY_SP, LIT_CHAR((*self) as u32)))
    }
}

impl ToTokens for bool {
    fn to_tokens(&self, _: &ExtCtxt) -> Vec<TokenTree> {
        vec!(TTTok(DUMMY_SP, IDENT(token::str_to_ident(self.to_str()), false)))
    }
}

fn mk_match_insts(cx: &mut ExtCtxt, sp: Span, arms: Vec<ast::Arm>) -> @Expr {
    let mat_pc = quote_expr!(&*cx, pc);
    as_expr(sp, ast::ExprMatch(mat_pc, arms))
}

fn mk_inst_arm(cx: &mut ExtCtxt, sp: Span, pc: uint, body: @Expr) -> ast::Arm {
    ast::Arm {
        pats: vec!(@ast::Pat{
            id: DUMMY_NODE_ID,
            span: sp,
            node: ast::PatLit(quote_expr!(&*cx, $pc)),
        }),
        guard: None,
        body: body,
    }
}

fn mk_any_arm(cx: &mut ExtCtxt, sp: Span, e: @Expr) -> ast::Arm {
    ast::Arm {
        pats: vec!(@ast::Pat{
            id: DUMMY_NODE_ID,
            span: sp,
            node: ast::PatWild,
        }),
        guard: None,
        body: e,
    }
}

fn mk_match_class(cx: &mut ExtCtxt, sp: Span,
                  casei: bool, ranges: &[(char, char)]) -> @Expr {
    let mut arms = ranges.iter().map(|&(mut start, mut end)| {
        if casei {
            start = start.to_uppercase();
            end = end.to_uppercase();
        }
        ast::Arm {
            pats: vec!(@ast::Pat{
                id: DUMMY_NODE_ID,
                span: sp,
                node: ast::PatRange(quote_expr!(&*cx, $start),
                                    quote_expr!(&*cx, $end)),
            }),
            guard: None,
            body: quote_expr!(&*cx, true),
        }
    }).collect::<Vec<ast::Arm>>();

    let nada = quote_expr!(&*cx, false);
    arms.push(mk_any_arm(cx, sp, nada));

    let match_on = quote_expr!(&*cx, c);
    as_expr(sp, ast::ExprMatch(match_on, arms))
}

fn mk_step_insts(cx: &mut ExtCtxt, sp: Span, re: &Dynamic) -> @Expr {
    let mut arms = re.p.insts.as_slice().iter().enumerate().map(|(pc, inst)| {
        let nextpc = pc + 1;
        let body = match *inst {
            Match => {
                quote_expr!(&*cx, {
                    match self.which {
                        Exists => {
                            return StepMatchEarlyReturn
                        }
                        Location => {
                            groups[0] = caps[0];
                            groups[1] = caps[1];
                            return StepMatch
                        }
                        Submatches => {
                            unsafe { groups.copy_memory(caps.as_slice()) }
                            return StepMatch
                        }
                    }
                })
            }
            OneChar(c, flags) => {
                if flags & FLAG_NOCASE > 0 {
                    let upc = c.to_uppercase();
                    quote_expr!(&*cx, {
                        if self.chars.prev.map(|c| c.to_uppercase()) == Some($upc) {
                            self.add(nlist, $nextpc, caps);
                        }
                    })
                } else {
                    quote_expr!(&*cx, {
                        if self.chars.prev == Some($c) {
                            self.add(nlist, $nextpc, caps);
                        }
                    })
                }
            }
            CharClass(ref ranges, flags) => {
                let negate = flags & FLAG_NEGATED > 0;
                let casei = flags & FLAG_NOCASE > 0;
                let get_char =
                    if casei {
                        quote_expr!(&*cx, self.chars.prev.unwrap().to_uppercase())
                    } else {
                        quote_expr!(&*cx, self.chars.prev.unwrap())
                    };
                let negcond =
                    if negate {
                        quote_expr!(&*cx, !found)
                    } else {
                        quote_expr!(&*cx, found)
                    };
                let match_ranges = mk_match_class(cx, sp,
                                                  casei, ranges.as_slice());
                quote_expr!(&*cx, {
                    if self.chars.prev.is_some() {
                        let c = $get_char;
                        let found = $match_ranges;
                        if $negcond {
                            self.add(nlist, $nextpc, caps);
                        }
                    }
                })
            }
            Any(flags) => {
                if flags & FLAG_DOTNL > 0 {
                    quote_expr!(&*cx, self.add(nlist, $nextpc, caps))
                } else {
                    let nl = '\n'; // no char lits allowed? wtf?
                    quote_expr!(&*cx, {
                        if self.chars.prev != Some($nl) {
                            self.add(nlist, $nextpc, caps)
                        }
                    })
                }
            }
            // For EmptyBegin, EmptyEnd, EmptyWordBoundary, Save, Jump, Split
            _ => quote_expr!(&*cx, {}),
        };
        mk_inst_arm(cx, sp, pc, body)
    }).collect::<Vec<ast::Arm>>();

    let nada = quote_expr!(&*cx, {});
    arms.push(mk_any_arm(cx, sp, nada));
    let m = mk_match_insts(cx, sp, arms);
    m
}

fn mk_add_insts(cx: &mut ExtCtxt, sp: Span, re: &Dynamic) -> @Expr {
    let mut arms = re.p.insts.as_slice().iter().enumerate().map(|(pc, inst)| {
        let nextpc = pc + 1;
        let body = match *inst {
            EmptyBegin(flags) => {
                let nl = '\n';
                let cond =
                    if flags & FLAG_MULTI > 0 {
                        quote_expr!(&*cx,
                            self.is_begin() || self.chars.prev == Some($nl)
                        )
                    } else {
                        quote_expr!(&*cx, self.is_begin())
                    };
                quote_expr!(&*cx, {
                    nlist.add($pc, groups, true);
                    if $cond { self.add(nlist, $nextpc, groups) }
                })
            }
            EmptyEnd(flags) => {
                let nl = '\n';
                let cond =
                    if flags & FLAG_MULTI > 0 {
                        quote_expr!(&*cx,
                            self.is_end() || self.chars.cur == Some($nl)
                        )
                    } else {
                        quote_expr!(&*cx, self.is_end())
                    };
                quote_expr!(&*cx, {
                    nlist.add($pc, groups, true);
                    if $cond { self.add(nlist, $nextpc, groups) }
                })
            }
            EmptyWordBoundary(flags) => {
                let cond =
                    if flags & FLAG_NEGATED > 0 {
                        quote_expr!(&*cx, !self.is_word_boundary())
                    } else {
                        quote_expr!(&*cx, self.is_word_boundary())
                    };
                quote_expr!(&*cx, {
                    nlist.add($pc, groups, true);
                    if $cond { self.add(nlist, $nextpc, groups) }
                })
            }
            Save(slot) => {
                // If this is saving a submatch location but we request
                // existence or only full match location, then we can skip
                // right over it every time.
                if slot > 1 {
                    quote_expr!(&*cx, {
                        nlist.add($pc, groups, true);
                        match self.which {
                            Submatches => {
                                let old = groups[$slot];
                                groups[$slot] = Some(self.ic);
                                self.add(nlist, $nextpc, groups);
                                groups[$slot] = old;
                            }
                            Exists | Location => self.add(nlist, $nextpc, groups),
                        }
                    })
                } else {
                    quote_expr!(&*cx, {
                        nlist.add($pc, groups, true);
                        match self.which {
                            Submatches | Location => {
                                let old = groups[$slot];
                                groups[$slot] = Some(self.ic);
                                self.add(nlist, $nextpc, groups);
                                groups[$slot] = old;
                            }
                            Exists => self.add(nlist, $nextpc, groups),
                        }
                    })
                }
            }
            Jump(to) => {
                quote_expr!(&*cx, {
                    nlist.add($pc, groups, true);
                    self.add(nlist, $to, groups);
                })
            }
            Split(x, y) => {
                quote_expr!(&*cx, {
                    nlist.add($pc, groups, true);
                    self.add(nlist, $x, groups);
                    self.add(nlist, $y, groups);
                })
            }
            // For Match, OneChar, CharClass, Any
            _ => quote_expr!(&*cx, nlist.add($pc, groups, false)),
        };
        mk_inst_arm(cx, sp, pc, body)
    }).collect::<Vec<ast::Arm>>();

    let nada = quote_expr!(&*cx, {});
    arms.push(mk_any_arm(cx, sp, nada));
    let m = mk_match_insts(cx, sp, arms);
    m
}

fn mk_check_prefix(cx: &mut ExtCtxt, sp: Span, re: &Dynamic) -> @Expr {
    if re.p.prefix.len() == 0 {
        quote_expr!(&*cx, {})
    } else {
        let bytes = as_expr_vec(cx, sp, re.p.prefix.as_slice().as_bytes(),
                                |cx, _, b| quote_expr!(&*cx, $b));
        quote_expr!(&*cx,
            if clist.size == 0 {
                let haystack = self.input.as_bytes().slice_from(self.ic);
                match find_prefix($bytes, haystack) {
                    None => break,
                    Some(i) => {
                        self.ic += i;
                        next_ic = self.chars.set(self.ic);
                    }
                }
            }
        )
    }
}

fn vec_from_fn(cx: &mut ExtCtxt, sp: Span, len: uint,
               to_expr: |&mut ExtCtxt| -> @Expr) -> @Expr {
    as_expr_vec(cx, sp, Vec::from_elem(len, ()).as_slice(),
                |cx, _, _| to_expr(cx))
}

fn vec_from_elem(cx: &mut ExtCtxt, sp: Span, len: uint, rep: @Expr) -> @Expr {
    as_expr_vec(cx, sp, Vec::from_elem(len, ()).as_slice(), |_, _, _| rep)
}

fn as_expr_vec<T>(cx: &mut ExtCtxt, sp: Span, xs: &[T],
                  to_expr: |&mut ExtCtxt, Span, &T| -> @Expr) -> @Expr {
    let mut exprs = vec!();
    // xs.iter() doesn't work here for some reason. No idea why.
    for i in ::std::iter::range(0, xs.len()) {
        exprs.push(to_expr(&mut *cx, sp, &xs[i]))
    }
    let vec_exprs = as_expr(sp, ExprVec(exprs));
    quote_expr!(&*cx, $vec_exprs)
}

fn as_expr(sp: Span, e: Expr_) -> @Expr {
    @Expr {
        id: DUMMY_NODE_ID,
        node: e,
        span: sp,
    }
}

fn parse(cx: &mut ExtCtxt, tts: &[TokenTree]) -> Option<~str> {
    let mut parser = parse::new_parser_from_tts(cx.parse_sess(), cx.cfg(),
                                                Vec::from_slice(tts));
    let entry = parser.parse_expr();
    let regex = match entry.node {
        ExprLit(lit) => {
            match lit.node {
                LitStr(ref s, _) => s.to_str(),
                _ => {
                    cx.span_err(entry.span, format!(
                        "expected string literal but got `{}`",
                        pprust::lit_to_str(lit)));
                    return None
                }
            }
        }
        _ => {
            cx.span_err(entry.span, format!(
                "expected string literal but got `{}`",
                pprust::expr_to_str(entry)));
            return None
        }
    };
    if !parser.eat(&EOF) {
        cx.span_err(parser.span, "only one string literal allowed");
        return None;
    }
    Some(regex)
}

fn parse_with_name(cx: &mut ExtCtxt, tts: &[TokenTree])
                  -> Option<(ast::Ident, ~str)> {
    let mut parser = parse::new_parser_from_tts(cx.parse_sess(), cx.cfg(),
                                                Vec::from_slice(tts));
    let entry = parser.parse_expr();
    let type_name = match entry.node {
        ast::ExprPath(ref p) => {
            if p.segments.len() != 1 || p.global {
                cx.span_err(entry.span, format!(
                    "expected valid type name but got `{}`",
                    pprust::expr_to_str(entry)));
                return None
            }
            p.segments.get(0).identifier.clone()
        }
        _ => {
            cx.span_err(entry.span, format!(
                "expected valid type name but got `{}`",
                pprust::expr_to_str(entry)));
            return None
        }
    };

    match parser.token {
        COMMA => parser.bump(),
        _ => {
            cx.span_err(entry.span, format!(
                "expected comma but got `{:?}`", parser.token));
        }
    }

    let entry = parser.parse_expr();
    let regex = match entry.node {
        ExprLit(lit) => {
            match lit.node {
                LitStr(ref s, _) => s.to_str(),
                _ => {
                    cx.span_err(entry.span, format!(
                        "expected string literal but got `{}`",
                        pprust::lit_to_str(lit)));
                    return None
                }
            }
        }
        _ => {
            cx.span_err(entry.span, format!(
                "expected string literal but got `{}`",
                pprust::expr_to_str(entry)));
            return None
        }
    };
    if !parser.eat(&EOF) {
        cx.span_err(parser.span, "only one string literal allowed");
        return None;
    }
    Some((type_name, regex))
}
