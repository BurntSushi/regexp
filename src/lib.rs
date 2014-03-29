#[crate_id = "regexp#0.1.0"];
#[crate_type = "lib"];
#[license = "UNLICENSE"];
#[doc(html_root_url = "http://burntsushi.net/rustdoc/regexp")];

//! Regular expressions for Rust.

#[feature(phase)];

#[phase(syntax, link)]
extern crate log;

mod parse;

