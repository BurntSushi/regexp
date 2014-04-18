// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[test]
fn re_replace() {
    let names =
        regexp!(r"(?P<first>\S+)\s+(?P<last>\S+)(?P<space>\s*)");
    let result = names.replace_all("w1 w2 w3 w4", "$last $first$space");
    assert_eq!(result, ~"w2 w1 w4 w3");
}
