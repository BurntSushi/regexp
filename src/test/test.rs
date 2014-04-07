use super::super::Regexp;

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
