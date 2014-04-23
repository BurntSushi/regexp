// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_id = "regexp_macros#0.11-pre"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]
#![license = "MIT/ASL2"]
#![doc(html_logo_url = "http://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "http://www.rust-lang.org/favicon.ico",
       html_root_url = "http://static.rust-lang.org/doc/master")]

#![feature(macro_registrar, managed_boxes, quote)]

//! This crate provides the `regexp!` macro. Its use is documented in the 
//! `regexp` crate.

extern crate regexp;
extern crate syntax;

use syntax::ast;
use syntax::codemap;
use syntax::ext::base::{
    SyntaxExtension, ExtCtxt, MacResult, MacExpr, DummyResult,
    NormalTT, BasicMacroExpander,
};
use syntax::parse;
use syntax::parse::token;
use syntax::print::pprust;

use regexp::Regexp;
use regexp::native::{
    OneChar, CharClass, Any, Save, Jump, Split,
    Match, EmptyBegin, EmptyEnd, EmptyWordBoundary,
    Program, Dynamic, Native,
    FLAG_NOCASE, FLAG_MULTI, FLAG_DOTNL, FLAG_NEGATED,
};

/// For the `regexp!` syntax extension. Do not use.
#[macro_registrar]
pub fn macro_registrar(register: |ast::Name, SyntaxExtension|) {
    let expander = ~BasicMacroExpander { expander: native, span: None };
    register(token::intern("regexp"), NormalTT(expander, None))
}

/// Generates specialized code for the Pike VM for a particular regular
/// expression.
///
/// There are two primary differences between the code generated here and the
/// general code in vm.rs.
///
/// 1. All heap allocation is removed. Sized vector types are used instead.
///    Care must be taken to make sure that these vectors are not copied
///    gratuitously. (If you're not sure, run the benchmarks. They will yell
///    at you if you do.)
/// 2. The main `match instruction { ... }` expressions are replaced with more
///    direct `match pc { ... }`. The generators can be found in 
///    `step_insts` and `add_insts`.
///
/// Other more minor changes include eliding code when possible (although this
/// isn't completely thorough at the moment), and translating character class
/// matching from using a binary search to a simple `match` expression (see
/// `match_class`).
///
/// It is strongly recommended to read the dynamic implementation in vm.rs
/// first before trying to understand the code generator. The implementation
/// strategy is identical and vm.rs has comments and will be easier to follow.
fn native(cx: &mut ExtCtxt, sp: codemap::Span, tts: &[ast::TokenTree])
         -> ~MacResult {
    let regex = match parse(cx, tts) {
        Some(r) => r,
        // error is logged in 'parse' with cx.span_err
        None => return DummyResult::any(sp),
    };
    let re = match Regexp::new(regex.to_owned()) {
        Ok(re) => re,
        Err(err) => {
            cx.span_err(sp, err.to_str());
            return DummyResult::any(sp)
        }
    };
    let prog = match re.p {
        Dynamic(ref prog) => prog.clone(),
        Native(_) => unreachable!(),
    };

    let mut gen = NfaGen {
        cx: cx, sp: sp, prog: prog,
        names: re.names.clone(), original: re.original.clone(),
    };
    MacExpr::new(gen.code())
}

struct NfaGen<'a, 'c> {
    cx: &'a mut ExtCtxt<'c>,
    sp: codemap::Span,
    prog: Program,
    names: ~[Option<~str>],
    original: ~str,
}

impl<'a, 'c> NfaGen<'a, 'c> {
    fn code(&mut self) -> @ast::Expr {
        // Most or all of the following things are used in the quasiquoted
        // expression returned.
        let num_cap_locs = 2 * self.prog.num_captures();
        let num_insts = self.prog.insts.len();
        let cap_names = self.vec_expr(self.names,
            |cx, name| match name {
                &Some(ref name) => {
                    let name = name.as_slice();
                    quote_expr!(cx, Some(~$name))
                }
                &None => quote_expr!(cx, None),
            }
        );
        let prefix_anchor = 
            match self.prog.insts.as_slice()[1] {
                EmptyBegin(flags) if flags & FLAG_MULTI == 0 => true,
                _ => false,
            };
        let init_groups = self.vec_from_fn(num_cap_locs,
                                           |cx| quote_expr!(&*cx, None));
        let prefix_bytes = self.vec_expr(self.prog.prefix.as_slice().as_bytes(),
                                         |cx, b| quote_expr!(&*cx, $b));
        let check_prefix = self.check_prefix();
        let step_insts = self.step_insts();
        let add_insts = self.add_insts();
        let regex = self.original.as_slice();

        quote_expr!(&*self.cx, {
fn exec<'t>(which: ::regexp::native::MatchKind, input: &'t str,
            start: uint, end: uint) -> Vec<Option<uint>> {
    #![allow(unused_imports)]
    use regexp::native::{
        MatchKind, Exists, Location, Submatches,
        StepState, StepMatchEarlyReturn, StepMatch, StepContinue,
        CharReader, find_prefix,
    };

    return Nfa {
        which: which,
        input: input,
        ic: 0,
        chars: CharReader::new(input),
    }.run(start, end);

    type Captures = [Option<uint>, ..$num_cap_locs];

    struct Nfa<'t> {
        which: MatchKind,
        input: &'t str,
        ic: uint,
        chars: CharReader<'t>,
    }

    impl<'t> Nfa<'t> {
        #[allow(unused_variable)]
        fn run(&mut self, start: uint, end: uint) -> Vec<Option<uint>> {
            let mut matched = false;
            let prefix_bytes: &[u8] = &$prefix_bytes;
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
                        StepMatchEarlyReturn =>
                            return vec![Some(0u), Some(0u)],
                        StepMatch => { matched = true; clist.empty() },
                        StepContinue => {},
                    }
                    i += 1;
                }
                ::std::mem::swap(&mut clist, &mut nlist);
                nlist.empty();
            }
            match self.which {
                Exists if matched     => vec![Some(0u), Some(0u)],
                Exists                => vec![None, None],
                Location | Submatches => {
                    let elts = groups.len();
                    let mut v = Vec::with_capacity(elts);
                    unsafe {
                        v.set_len(elts);
                        ::std::ptr::copy_nonoverlapping_memory(
                            v.as_mut_ptr(), groups.as_ptr(), elts);
                    }
                    v
                }
            }
        }

        // Sometimes `nlist` is never used (for empty regexes).
        #[allow(unused_variable)]
        #[inline(always)]
        fn step(&self, groups: &mut Captures, nlist: &mut Threads,
                caps: &mut Captures, pc: uint) -> StepState {
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

        #[inline(always)]
        fn add(&mut self, pc: uint, groups: &Captures) {
            let t = &mut self.queue[self.size];
            t.pc = pc;
            match self.which {
                Exists => {},
                Location => {
                    t.groups[0] = groups[0];
                    t.groups[1] = groups[1];
                }
                Submatches => {
                    unsafe { t.groups.copy_memory(groups.as_slice()) }
                }
            }
            self.sparse[pc] = self.size;
            self.size += 1;
        }

        #[inline(always)]
        fn add_empty(&mut self, pc: uint) {
            self.queue[self.size].pc = pc;
            self.sparse[pc] = self.size;
            self.size += 1;
        }

        #[inline(always)]
        fn contains(&self, pc: uint) -> bool {
            let s = self.sparse[pc];
            s < self.size && self.queue[s].pc == pc
        }

        #[inline(always)]
        fn empty(&mut self) {
            self.size = 0;
        }

        #[inline(always)]
        fn pc(&self, i: uint) -> uint {
            self.queue[i].pc
        }

        #[inline(always)]
        fn groups<'r>(&'r mut self, i: uint) -> &'r mut Captures {
            &'r mut self.queue[i].groups
        }
    }
}

::regexp::Regexp {
    original: ~$regex,
    names: ~$cap_names,
    p: ::regexp::native::Native(exec),
}
        })
    }

    // Generates code for the `add` method, which is responsible for adding
    // zero-width states to the next queue of states to visit.
    fn add_insts(&self) -> @ast::Expr {
        let arms = self.prog.insts.iter().enumerate().map(|(pc, inst)| {
            let nextpc = pc + 1;
            let body = match *inst {
                EmptyBegin(flags) => {
                    let nl = '\n';
                    let cond =
                        if flags & FLAG_MULTI > 0 {
                            quote_expr!(&*self.cx,
                                self.chars.is_begin()
                                || self.chars.prev == Some($nl)
                            )
                        } else {
                            quote_expr!(&*self.cx, self.chars.is_begin())
                        };
                    quote_expr!(&*self.cx, {
                        nlist.add_empty($pc);
                        if $cond { self.add(nlist, $nextpc, &mut *groups) }
                    })
                }
                EmptyEnd(flags) => {
                    let nl = '\n';
                    let cond =
                        if flags & FLAG_MULTI > 0 {
                            quote_expr!(&*self.cx,
                                self.chars.is_end()
                                || self.chars.cur == Some($nl)
                            )
                        } else {
                            quote_expr!(&*self.cx, self.chars.is_end())
                        };
                    quote_expr!(&*self.cx, {
                        nlist.add_empty($pc);
                        if $cond { self.add(nlist, $nextpc, &mut *groups) }
                    })
                }
                EmptyWordBoundary(flags) => {
                    let cond =
                        if flags & FLAG_NEGATED > 0 {
                            quote_expr!(&*self.cx, !self.chars.is_word_boundary())
                        } else {
                            quote_expr!(&*self.cx, self.chars.is_word_boundary())
                        };
                    quote_expr!(&*self.cx, {
                        nlist.add_empty($pc);
                        if $cond { self.add(nlist, $nextpc, &mut *groups) }
                    })
                }
                Save(slot) => {
                    // If this is saving a submatch location but we request
                    // existence or only full match location, then we can skip
                    // right over it every time.
                    if slot > 1 {
                        quote_expr!(&*self.cx, {
                            nlist.add_empty($pc);
                            match self.which {
                                Submatches => {
                                    let old = groups[$slot];
                                    groups[$slot] = Some(self.ic);
                                    self.add(nlist, $nextpc, &mut *groups);
                                    groups[$slot] = old;
                                }
                                Exists | Location =>
                                    self.add(nlist, $nextpc, &mut *groups),
                            }
                        })
                    } else {
                        quote_expr!(&*self.cx, {
                            nlist.add_empty($pc);
                            match self.which {
                                Submatches | Location => {
                                    let old = groups[$slot];
                                    groups[$slot] = Some(self.ic);
                                    self.add(nlist, $nextpc, &mut *groups);
                                    groups[$slot] = old;
                                }
                                Exists =>
                                    self.add(nlist, $nextpc, &mut *groups),
                            }
                        })
                    }
                }
                Jump(to) => {
                    quote_expr!(&*self.cx, {
                        nlist.add_empty($pc);
                        self.add(nlist, $to, &mut *groups);
                    })
                }
                Split(x, y) => {
                    quote_expr!(&*self.cx, {
                        nlist.add_empty($pc);
                        self.add(nlist, $x, &mut *groups);
                        self.add(nlist, $y, &mut *groups);
                    })
                }
                // For Match, OneChar, CharClass, Any
                _ => quote_expr!(&*self.cx, nlist.add($pc, &*groups)),
            };
            self.arm_inst(pc, body)
        }).collect::<Vec<ast::Arm>>();

        self.match_insts(arms)
    }

    // Generates the code for the `step` method, which processes all states
    // in the current queue that consume a single character.
    fn step_insts(&self) -> @ast::Expr {
        let arms = self.prog.insts.iter().enumerate().map(|(pc, inst)| {
            let nextpc = pc + 1;
            let body = match *inst {
                Match => {
                    quote_expr!(&*self.cx, {
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
                        quote_expr!(&*self.cx, {
                            let upc = self.chars.prev.map(|c| c.to_uppercase());
                            if upc == Some($upc) {
                                self.add(nlist, $nextpc, caps);
                            }
                        })
                    } else {
                        quote_expr!(&*self.cx, {
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
                            quote_expr!(&*self.cx, self.chars.prev.unwrap().to_uppercase())
                        } else {
                            quote_expr!(&*self.cx, self.chars.prev.unwrap())
                        };
                    let negcond =
                        if negate {
                            quote_expr!(&*self.cx, !found)
                        } else {
                            quote_expr!(&*self.cx, found)
                        };
                    let mranges = self.match_class(casei, ranges.as_slice());
                    quote_expr!(&*self.cx, {
                        if self.chars.prev.is_some() {
                            let c = $get_char;
                            let found = $mranges;
                            if $negcond {
                                self.add(nlist, $nextpc, caps);
                            }
                        }
                    })
                }
                Any(flags) => {
                    if flags & FLAG_DOTNL > 0 {
                        quote_expr!(&*self.cx, self.add(nlist, $nextpc, caps))
                    } else {
                        let nl = '\n'; // no char lits allowed? wtf?
                        quote_expr!(&*self.cx, {
                            if self.chars.prev != Some($nl) {
                                self.add(nlist, $nextpc, caps)
                            }
                        })
                    }
                }
                // EmptyBegin, EmptyEnd, EmptyWordBoundary, Save, Jump, Split
                _ => quote_expr!(&*self.cx, {}),
            };
            self.arm_inst(pc, body)
        }).collect::<Vec<ast::Arm>>();

        self.match_insts(arms)
    }

    // Translates a character class into a match expression.
    // This avoids a binary search (and is hopefully replaced by a jump
    // table).
    fn match_class(&self, casei: bool, ranges: &[(char, char)]) -> @ast::Expr {
        let mut arms = ranges.iter().map(|&(mut start, mut end)| {
            if casei {
                start = start.to_uppercase();
                end = end.to_uppercase();
            }
            ast::Arm {
                pats: vec!(@ast::Pat{
                    id: ast::DUMMY_NODE_ID,
                    span: self.sp,
                    node: ast::PatRange(quote_expr!(&*self.cx, $start),
                                        quote_expr!(&*self.cx, $end)),
                }),
                guard: None,
                body: quote_expr!(&*self.cx, true),
            }
        }).collect::<Vec<ast::Arm>>();

        arms.push(self.wild_arm_expr(quote_expr!(&*self.cx, false)));

        let match_on = quote_expr!(&*self.cx, c);
        self.dummy_expr(ast::ExprMatch(match_on, arms))
    }

    // Generates code for checking a literal prefix of the search string.
    // The code is only generated if the regexp *has* a literal prefix.
    // Otherwise, a no-op is returned.
    fn check_prefix(&self) -> @ast::Expr {
        if self.prog.prefix.len() == 0 {
            quote_expr!(&*self.cx, {})
        } else {
            quote_expr!(&*self.cx,
                if clist.size == 0 {
                    let haystack = self.input.as_bytes().slice_from(self.ic);
                    match find_prefix(prefix_bytes, haystack) {
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

    // Builds a `match pc { ... }` expression from a list of arms, specifically
    // for matching the current program counter with an instruction.
    // A wild-card arm is automatically added that executes a no-op. It will
    // never be used, but is added to satisfy the compiler complaining about
    // non-exhaustive patterns.
    fn match_insts(&self, mut arms: Vec<ast::Arm>) -> @ast::Expr {
        let mat_pc = quote_expr!(&*self.cx, pc);
        arms.push(self.wild_arm_expr(quote_expr!(&*self.cx, {})));
        self.dummy_expr(ast::ExprMatch(mat_pc, arms))
    }

    // Creates a match arm for the instruction at `pc` with the expression
    // `body`.
    fn arm_inst(&self, pc: uint, body: @ast::Expr) -> ast::Arm {
        ast::Arm {
            pats: vec!(@ast::Pat{
                id: ast::DUMMY_NODE_ID,
                span: self.sp,
                node: ast::PatLit(quote_expr!(&*self.cx, $pc)),
            }),
            guard: None,
            body: body,
        }
    }

    // Creates a wild-card match arm with the expression `body`.
    fn wild_arm_expr(&self, body: @ast::Expr) -> ast::Arm {
        ast::Arm {
            pats: vec!(@ast::Pat{
                id: ast::DUMMY_NODE_ID,
                span: self.sp,
                node: ast::PatWild,
            }),
            guard: None,
            body: body,
        }
    }

    // Builds a `[a, b, .., len]` expression where each element is the result
    // of executing `to_expr`.
    fn vec_from_fn(&self, len: uint, to_expr: |&ExtCtxt| -> @ast::Expr)
                  -> @ast::Expr {
        self.vec_expr(Vec::from_elem(len, ()).as_slice(),
                      |cx, _| to_expr(cx))
    }

    // Converts `xs` to a `[x1, x2, .., xN]` expression by calling `to_expr`
    // on each element in `xs`.
    fn vec_expr<T>(&self, xs: &[T], to_expr: |&ExtCtxt, &T| -> @ast::Expr)
                  -> @ast::Expr {
        let mut exprs = vec!();
        for x in xs.iter() {
            exprs.push(to_expr(self.cx, x))
        }
        let vec_exprs = self.dummy_expr(ast::ExprVec(exprs));
        quote_expr!(&*self.cx, $vec_exprs)
    }

    // Creates an expression with a dummy node ID given an underlying
    // `ast::Expr_`.
    fn dummy_expr(&self, e: ast::Expr_) -> @ast::Expr {
        @ast::Expr {
            id: ast::DUMMY_NODE_ID,
            node: e,
            span: self.sp,
        }
    }
}

// This trait is defined in the quote module in the syntax crate, but I
// don't think it's exported.
// Interestingly, quote_expr! only requires that a 'to_tokens' method be
// defined rather than satisfying a particular trait.
#[doc(hidden)]
trait ToTokens {
    fn to_tokens(&self, cx: &ExtCtxt) -> Vec<ast::TokenTree>;
}

impl ToTokens for char {
    fn to_tokens(&self, _: &ExtCtxt) -> Vec<ast::TokenTree> {
        vec!(ast::TTTok(codemap::DUMMY_SP, token::LIT_CHAR((*self) as u32)))
    }
}

impl ToTokens for bool {
    fn to_tokens(&self, _: &ExtCtxt) -> Vec<ast::TokenTree> {
        let ident = token::IDENT(token::str_to_ident(self.to_str()), false);
        vec!(ast::TTTok(codemap::DUMMY_SP, ident))
    }
}

/// Looks for a single string literal and returns it.
/// Otherwise, logs an error with cx.span_err and returns None.
fn parse(cx: &mut ExtCtxt, tts: &[ast::TokenTree]) -> Option<~str> {
    let mut parser = parse::new_parser_from_tts(cx.parse_sess(), cx.cfg(),
                                                Vec::from_slice(tts));
    let entry = cx.expand_expr(parser.parse_expr());
    let regex = match entry.node {
        ast::ExprLit(lit) => {
            match lit.node {
                ast::LitStr(ref s, _) => s.to_str(),
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
    if !parser.eat(&token::EOF) {
        cx.span_err(parser.span, "only one string literal allowed");
        return None;
    }
    Some(regex)
}
