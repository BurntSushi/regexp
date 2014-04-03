#![crate_id = "regexp#0.1.0"]
#![crate_type = "lib"]
#![license = "UNLICENSE"]
#![doc(html_root_url = "http://burntsushi.net/rustdoc/regexp")]

#![allow(unused_imports)]
#![allow(dead_code)]

//! Regular expressions for Rust.

#![feature(phase)]

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
        write!(f.buf, "{} error at position {}: {}",
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

        let insts = compile::compile(re);
        debug!("Insts: {}", insts);

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
        run("(a*?)+", "aab");
        // run("(?sm)(.*?)^ab", "\n\n\nab"); 
        // run("(?sm)ab$\n", "ab\n"); 
    }

    #[test]
    #[ignore]
    fn captures() {
        run("(a)b", "ab");
        run("(?sm)(.*)^\nab", "\n\n\nab");
    }
}
