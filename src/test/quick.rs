use rand::task_rng;
use std::str;
use quickcheck::{Arbitrary, Config, Gen, Iter, gen, quickcheck_config};
use super::super::Regexp;
use super::gen_regex_str;

static MANY_TESTS: Config = Config { tests: 100, max_tests: 10000 };

#[deriving(Clone, Show)]
struct RegexStr(~str);

impl Arbitrary for RegexStr {
    fn arbitrary<G: Gen>(g: &mut G) -> RegexStr {
        let size = g.size();
        RegexStr(gen_regex_str(g, size))
    }

    fn shrink(&self) -> ~Iter<RegexStr> {
        let &RegexStr(ref s) = self;
        ~s.shrink().map(RegexStr) as ~Iter<RegexStr>
    }
}

#[test]
fn no_crashing_ascii() {
    fn prop(s: ~str) -> bool {
        let _ = Regexp::new(s);
        true
    }
    quickcheck_config(MANY_TESTS, &mut gen(task_rng(), 100), prop);
}

#[test]
fn no_crashing_chars() {
    fn prop(cs: Vec<char>) -> bool {
        let s = str::from_chars(cs.as_slice());
        let _ = Regexp::new(s);
        true
    }
    quickcheck_config(MANY_TESTS, &mut gen(task_rng(), 100), prop);
}

#[test]
fn no_crashing_regex_chars() {
    fn prop(RegexStr(s): RegexStr) -> bool {
        let _ = Regexp::new(s);
        true
    }
    quickcheck_config(MANY_TESTS, &mut gen(task_rng(), 100), prop);
}
