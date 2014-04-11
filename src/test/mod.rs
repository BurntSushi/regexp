use super::{Regexp, NoExpand};

#[cfg(bench)]
mod bench;

#[test]
fn splitn() {
    let re = Regexp::new(r"\d+").unwrap();
    let text = "cauchy123plato456tyler789binx";
    let subs: Vec<&str> = re.splitn(text, 2).collect();
    assert_eq!(subs, vec!("cauchy", "plato456tyler789binx"));
}

#[test]
fn split() {
    let re = Regexp::new(r"\d+").unwrap();
    let text = "cauchy123plato456tyler789binx";
    let subs: Vec<&str> = re.split(text).collect();
    assert_eq!(subs, vec!("cauchy", "plato", "tyler", "binx"));
}

macro_rules! replace(
    ($name:ident, $which:ident, $re:expr,
     $search:expr, $replace:expr, $result:expr) => (
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
replace!(rep_trim, replace_all, "^[ \t]+|[ \t]+$", " \t  trim me\t   \t",
         "", ~"trim me")

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

// Some crazy expressions from regular-expressions.info.
mat!(match_ranges,
     r"\b(?:[0-9]|[1-9][0-9]|1[0-9][0-9]|2[0-4][0-9]|25[0-5])\b",
     "num: 255", Some((5, 8)))
mat!(match_ranges_not,
     r"\b(?:[0-9]|[1-9][0-9]|1[0-9][0-9]|2[0-4][0-9]|25[0-5])\b",
     "num: 256", None)
mat!(match_float1, r"[-+]?[0-9]*\.?[0-9]+", "0.1", Some((0, 3)))
mat!(match_float2, r"[-+]?[0-9]*\.?[0-9]+", "0.1.2", Some((0, 3)))
mat!(match_float3, r"[-+]?[0-9]*\.?[0-9]+", "a1.2", Some((1, 4)))
mat!(match_float4, r"^[-+]?[0-9]*\.?[0-9]+$", "1.a", None)
mat!(match_email, r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,4}\b",
     "mine is jam.slam@gmail.com ", Some((8, 26)))
mat!(match_email_not, r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,4}\b",
     "mine is jam.slam@gmail ", None)
mat!(match_email_big, r"[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?",
     "mine is jam.slam@gmail.com ", Some((8, 26)))
mat!(match_date1,
     r"^(19|20)\d\d[- /.](0[1-9]|1[012])[- /.](0[1-9]|[12][0-9]|3[01])$",
     "1900-01-01", Some((0, 10)))
mat!(match_date2,
     r"^(19|20)\d\d[- /.](0[1-9]|1[012])[- /.](0[1-9]|[12][0-9]|3[01])$",
     "1900-00-01", None)
mat!(match_date3,
     r"^(19|20)\d\d[- /.](0[1-9]|1[012])[- /.](0[1-9]|[12][0-9]|3[01])$",
     "1900-13-01", None)

mod matches;

mod large {
    use rand::{Rng, task_rng};
    use std::str;
    use super::super::compile::Program;
    use super::super::parse::parse;
    use super::super::quote;

    fn gen_regex_str<G: Rng>(g: &mut G, len: uint) -> ~str {
        static CHARSET: &'static [u8] =
            bytes!("ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                    abcdefghijklmnopqrstuvwxyz\
                    0123456789\
                    ~!@#$%^&*(){},.[]\n \\-+");
        let mut s = str::with_capacity(len);
        for _ in range(0, len) {
            s.push_char(g.choose(CHARSET) as char)
        }
        s
    }

    #[test]
    fn large_str_parse() {
        // Make sure large strings don't cause the parser to blow the stack.
        let g = &mut task_rng();
        let s = quote(gen_regex_str(g, 100000));
        let _ = parse(s);
    }

    #[test]
    fn large_str_compile() {
        // Make sure large strings don't cause the parser to blow the stack.
        // Note that we can go bigger here, but the parser gets really slow
        // at 1MB of data.
        let g = &mut task_rng();
        let s = quote(gen_regex_str(g, 100000));
        let _ = Program::new(s, parse(s).unwrap());
    }
}
