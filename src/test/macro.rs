#![crate_id = "regexp_re_test#0.1.0"]

#![feature(phase)]
extern crate regexp;
#[phase(syntax)] extern crate regexp_re;

use regexp::Regexp;

#[test]
fn replace() {
    static names: Regexp =
        re!(r"(?P<first>\S+)\s+(?P<last>\S+)(?P<space>\s*)");
    let result = names.replace_all("w1 w2 w3 w4", "$last $first$space");
    assert_eq!(result, ~"w2 w1 w4 w3");
}
