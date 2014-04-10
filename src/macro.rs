#![crate_id = "regexp_re#0.1.0"]
#![crate_type = "dylib"]
#![license = "UNLICENSE"]
#![doc(html_root_url = "http://burntsushi.net/rustdoc/regexp_re")]

#![feature(macro_rules, phase, macro_registrar, managed_boxes, quote)]

extern crate regexp;
extern crate syntax;

use syntax::ast::{Name, TokenTree, TTTok, DUMMY_NODE_ID};
use syntax::ast::{Expr, Expr_, ExprLit, LitStr, ExprVec};
use syntax::codemap::{Span, DUMMY_SP};
use syntax::ext::base::{SyntaxExtension,
                        ExtCtxt,
                        MacResult,
                        MRExpr,
                        NormalTT,
                        BasicMacroExpander};
use syntax::parse;
use syntax::parse::token;
use syntax::parse::token::{EOF, LIT_CHAR, IDENT};

use regexp::Regexp;
use regexp::program::{Program, MaybeStaticClass};
use regexp::program::{Inst, Char_, CharClass, Any_, Save, Jump, Split};
use regexp::program::{Match, EmptyBegin, EmptyEnd, EmptyWordBoundary};

/// For the `re!` syntax extension. Do not use.
#[macro_registrar]
pub fn macro_registrar(reg: |Name, SyntaxExtension|) {
    reg(token::intern("re"),
        NormalTT(~BasicMacroExpander {
            expander: re_static,
            span: None,
        },
        None));
}

fn re_static(cx: &mut ExtCtxt, sp: Span, tts: &[TokenTree]) -> MacResult {
    let restr = match parse_one_str_lit(cx, tts) {
        Some(re) => re,
        None => return MacResult::dummy_expr(sp),
    };
    let re = match Regexp::new(restr.to_owned()) {
        Ok(re) => re,
        Err(err) => {
            cx.span_err(sp, err.to_str());
            return MacResult::dummy_expr(sp)
        }
    };

    let insts = as_expr_vec_static(cx, sp, re.p.insts(), 
        |cx, sp, inst| inst_to_expr(cx, sp, inst));
    let names = as_expr_vec_static(cx, sp, re.p.names(),
        |cx, _, name| match name {
            &Some(ref name) => {
                let name = name.as_slice();
                quote_expr!(cx, Some(::std::str::Slice(&'static $name)))
            }
            &None => quote_expr!(cx, None),
        }
    );
    let prefix = as_expr_vec_static(cx, sp, re.p.prefix(),
        |cx, _, &c| quote_expr!(cx, $c));
    MRExpr(quote_expr!(&*cx,
        regexp::Regexp {
            p: &'static regexp::program::StaticProgram {
                regex: $restr,
                insts: $insts,
                names: $names,
                prefix: $prefix,
            },
        }
    ))
}

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

fn inst_to_expr(cx: &mut ExtCtxt, sp: Span, inst: &Inst) -> @Expr {
    match inst {
        &Match => quote_expr!(&*cx, regexp::program::Match),
        &Char_(c, casei) =>
            quote_expr!(&*cx, regexp::program::Char_($c, $casei)),
        &CharClass(ref ranges, negated, casei) => {
            char_class_to_expr(cx, sp, ranges, negated, casei)
        }
        &Any_(multi) =>
            quote_expr!(&*cx, regexp::program::Any($multi)),
        &EmptyBegin(multi) =>
            quote_expr!(&*cx, regexp::program::EmptyBegin($multi)),
        &EmptyEnd(multi) =>
            quote_expr!(&*cx, regexp::program::EmptyEnd($multi)),
        &EmptyWordBoundary(multi) =>
            quote_expr!(&*cx, regexp::program::EmptyWordBoundary($multi)),
        &Save(slot) =>
            quote_expr!(&*cx, regexp::program::Save($slot)),
        &Jump(pc) =>
            quote_expr!(&*cx, regexp::program::Jump($pc)),
        &Split(x, y) =>
            quote_expr!(&*cx, regexp::program::Split($x, $y)),
    }
}

fn char_class_to_expr(cx: &mut ExtCtxt, sp: Span,
                      ranges: &MaybeStaticClass,
                      negated: bool, casei: bool) -> @Expr {
    let ranges = as_expr_vec_static(cx, sp, ranges.as_slice(),
        |cx, _, &(x, y)| quote_expr!(&*cx, ($x, $y)));
    quote_expr!(&*cx,
        regexp::program::CharClass(
            regexp::program::StaticClass($ranges), $negated, $casei))
}

fn as_expr_vec_static<T>(cx: &mut ExtCtxt, sp: Span, xs: &[T],
                         to_expr: |&mut ExtCtxt, Span, &T| -> @Expr) -> @Expr {
    let mut exprs = vec!();
    for x in xs.iter() {
        exprs.push(to_expr(&mut *cx, sp, x))
    }
    let vec_exprs = as_expr(sp, ExprVec(exprs));
    quote_expr!(&*cx, &'static $vec_exprs)
}

fn as_expr(sp: Span, e: Expr_) -> @Expr {
    @Expr {
        id: DUMMY_NODE_ID,
        node: e,
        span: sp,
    }
}

fn parse_one_str_lit(cx: &mut ExtCtxt, tts: &[TokenTree]) -> Option<~str> {
    let mut parser = parse::new_parser_from_tts(cx.parse_sess(), cx.cfg(),
                                                Vec::from_slice(tts));
    let entry = parser.parse_expr();

    let lit = match entry.node {
        ExprLit(lit) => {
            match lit.node {
                LitStr(ref s, _) => s.clone(),
                _ => {
                    cx.span_err(entry.span, "expected string literal");
                    return None
                }
            }
        }
        _ => {
            cx.span_err(entry.span, "expected string literal");
            return None
        }
    };
    if !parser.eat(&EOF) {
        cx.span_err(parser.span, "only one string literal allowed");
        return None;
    }
    Some(lit.to_str().to_owned())
}
