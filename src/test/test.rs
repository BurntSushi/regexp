use super::super::{Regexp, NoExpand};

#[test]
fn splitn() {
    let re = Regexp::new(r"\d+").unwrap();
    let text = "cauchy123plato456tyler789binks";
    let subs: Vec<&str> = re.splitn(text, 2).collect();
    assert_eq!(subs, vec!("cauchy", "plato456tyler789binks"));
}

#[test]
fn split() {
    let re = Regexp::new(r"\d+").unwrap();
    let text = "cauchy123plato456tyler789binks";
    let subs: Vec<&str> = re.split(text).collect();
    assert_eq!(subs, vec!("cauchy", "plato", "tyler", "binks"));
}

macro_rules! replace(
    ($name:ident, $which:ident, $re:expr, $search:expr, $replace:expr, $result:expr) => (
        #[test]
        fn $name() {
            let re = Regexp::new($re).unwrap();
            assert_eq!(re.$which($search, $replace), $result);
        }
    );
)

replace!(rep_first, replace, r"\d", "age: 26", "Z", ~"age: Z6")
replace!(rep_plus, replace, r"\d+", "age: 26", "Z", ~"age: Z")
replace!(rep_all, replace_all, r"\d", "age: 26", "Z", ~"age: ZZ")
replace!(rep_groups, replace, r"(\S+)\s+(\S+)", "w1 w2", "$2 $1", ~"w2 w1")
replace!(rep_double_dollar, replace,
         r"(\S+)\s+(\S+)", "w1 w2", "$2 $$1", ~"w2 $1")
replace!(rep_no_expand, replace,
         r"(\S+)\s+(\S+)", "w1 w2", NoExpand("$2 $1"), ~"$2 $1")
replace!(rep_named, replace_all,
         r"(?P<first>\S+)\s+(?P<last>\S+)(?P<space>\s*)",
         "w1 w2 w3 w4", "$last $first$space", ~"w2 w1 w4 w3")

macro_rules! fail_parse(
    ($name:ident, $re:expr) => (
        #[test]
        fn $name() {
            let re = $re;
            match Regexp::new(re) {
                Err(_) => {},
                Ok(_) => fail!("Regexp '{}' should cause a parse error.", re),
            }
        }
    );
)

fail_parse!(fail_parse_double_repeat, "a**")
fail_parse!(fail_parse_no_repeat_arg, "*")
fail_parse!(fail_parse_no_repeat_arg_begin, "^*")
fail_parse!(fail_parse_incomplete_escape, "\\")
fail_parse!(fail_parse_class_incomplete, "[A-")
fail_parse!(fail_parse_class_not_closed, "[A")
fail_parse!(fail_parse_class_no_begin, r"[\A]")
fail_parse!(fail_parse_class_no_end, r"[\z]")
fail_parse!(fail_parse_class_no_boundary, r"[\b]")
fail_parse!(fail_parse_open_paren, "(")
fail_parse!(fail_parse_close_paren, ")")
fail_parse!(fail_parse_invalid_range, "[a-Z]")
fail_parse!(fail_parse_empty_capture_name, "(?P<>a)")
fail_parse!(fail_parse_empty_capture_exp, "(?P<name>)")
fail_parse!(fail_parse_bad_flag, "(?a)a")
fail_parse!(fail_parse_empty_alt_before, "|a")
fail_parse!(fail_parse_empty_alt_after, "a|")
fail_parse!(fail_parse_counted_big_exact, "a{1001}")
fail_parse!(fail_parse_counted_big_min, "a{1001,}")
fail_parse!(fail_parse_counted_no_close, "a{1001")

macro_rules! mat(
    ($name:ident, $re:expr, $text:expr, $($loc:tt)+) => (
        #[test]
        fn $name() {
            let re = $re;
            let text = $text;
            let expected: Vec<Option<(uint, uint)>> = vec!($($loc)+);
            let r = match Regexp::new(re) {
                Ok(r) => r,
                Err(err) => fail!("Could not compile '{}': {}", re, err),
            };
            let got = match r.captures(text) {
                Some(c) => c.iter_pos().collect::<Vec<Option<(uint, uint)>>>(),
                None => vec!(None),
            };
            // The test set sometimes leave out capture groups, so truncate
            // actual capture groups to match test set.
            let (sexpect, mut sgot) = (expected.as_slice(), got.as_slice());
            if sgot.len() > sexpect.len() {
                sgot = sgot.slice(0, sexpect.len())
            }
            if sexpect != sgot {
                fail!("For RE '{}' against '{}', expected '{}' but got '{}'",
                      re, text, sexpect, sgot);
            }
        }
    );
)

mod matches;
