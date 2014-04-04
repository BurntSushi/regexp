#![crate_id = "regexp#0.1.0"]
#![crate_type = "lib"]
#![license = "UNLICENSE"]
#![doc(html_root_url = "http://burntsushi.net/rustdoc/regexp")]

#![allow(unused_imports)]
#![allow(dead_code)]

//! Regular expressions for Rust.

#![feature(phase)]

extern crate collections;
#[phase(syntax, link)]
extern crate log;

use std::fmt;

mod compile;
mod parse;
mod vm;

pub struct Error {
    pub pos: uint,
    pub kind: ErrorKind,
    pub msg: ~str,
}

#[deriving(Show)]
pub enum ErrorKind {
    Bug,
    BadSyntax,
}

impl fmt::Show for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f.buf, "{} error near position {}: {}",
            self.kind, self.pos, self.msg)
    }
}

#[cfg(test)]
mod test {
    use super::compile;
    use super::parse;
    use super::vm;

    fn run(regexp: &str, text: &str) {
        debug!("\n--------------------------------");
        debug!("RE: {}", regexp);
        debug!("Text: {}", text);

        let re = match parse::parse(regexp) {
            Err(err) => fail!("{}", err),
            Ok(re) => re,
        };
        debug!("AST: {}", re);

        let (insts, cap_names) = compile::compile(re);
        debug!("Insts: {}", insts);
        debug!("Capture names: {}", cap_names);

        let matched = vm::run(insts, text);
        debug!("Matched: {}", matched);

        debug!("--------------------------------");
    }

    #[test]
    // #[ignore] 
    fn simple() {
        // run("(?i:and)rew", "aNdrew"); 
        // run("a+b+?", "abbbbb"); 
        // run("(?s:.+)", "abb\nbbb"); 
        // run("(a*)+", "aaa"); 
        // run("(?sm)(.*?)^ab", "\n\n\nab"); 
        // run("(?sm)ab$\n", "ab\n"); 
        // run("(a{2}){3}", "aaaaaa"); 
        // run("a{2,}", "aaaaaa"); 
        // run("[a-z0-9]+", "ab0cdef"); 
        // run("<([^>])+>", "<strong>hello there</strong>"); 
        // run("(a|bcdef|g|ab|c|d|e|efg|fg)*", "abcdefg"); 
        // run("[^[:^alnum:]]+", "abc0123"); 

        // run(r"[\D]+", "abc123abc"); 
        // run(r".*([a-z]\b)", "andrew gallant"); 
        // run(r"\**", "**"); 
        // run(r"[\A]+", "-]a^a-a"); 
        // run(r"[\p{N}\p{Cherokee}]+", "ᏑⅡᏡⅥ"); 
        // run(r"\pN+", "ⅡⅢⅳⅥ"); 
        run("(?i)[^a-z]+", "ANDREW");
    }

    #[test]
    // #[ignore] 
    fn captures() {
        // run("(a)b", "ab"); 
        // run("(?sm)(.*)^\nab", "\n\n\nab"); 
        // run(r"(?P<wat>\d+)a(?P<a>\d+)", "123a456"); 
    }
}
