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
    ($name:ident, $re:expr, $text:expr) => ( mat!($name, $re, $text,) );
    ($name:ident, $re:expr, $text:expr, $($loc:expr),+) => (
        #[test]
        fn $name() {
            let re = $re;
            let text = $text;
            let locs: Vec<Option<(uint, uint)>> = vec!($($loc)+);
            let r = match Regexp::new(re) {
                Ok(r) => r,
                Err(err) => fail!("Could not compile '{}': {}", re, err),
            };
            let test_locs = match r.captures(text) {
                Some(c) => c.iter_pos().collect::<Vec<Option<(uint, uint)>>>(),
                None => vec!(),
            };
            if locs != test_locs {
                fail!("For RE '{}' against '{}', expected '{}' but got '{}'",
                      re, text, locs, test_locs);
            }
        }
    );
)

// mat!(match_1, "abc", "abcabc", Some((0, 3))) 
// mat!(match_2, "(a*)*", "-", Some((0, 0)), None) 

fn print_matches(re: &str, text: &str) {
    let r = Regexp::new(re).unwrap();
    let caps = r.captures(text).unwrap();
    for (i, s) in caps.iter().enumerate() {
        debug!("{} :: '{}'", caps.pos(i), s);
    }
    debug!("--------------------------");
}

#[test]
fn wat() {
    debug!("");
    // print_matches("(a?)((ab)?)(b?)", "ab"); 
    // print_matches("((a?)((ab)?))(b?)", "ab"); 
    // print_matches(r"(^|[ (,;])((([Ff]eb[^ ]* *|0*2/|\* */?)0*[6-7]))([^0-9]|$)", 
                  // "feb 1,Feb 6"); 
    // print_matches("(a*)*", "-"); 
    // print_matches("(a*|b)*", "-"); 
    // print_matches("(a+|b)*", "ab"); 
    // print_matches("(aba|a*b)*", "ababa"); 
    // print_matches("(a(b)?)+", "aba"); 
    print_matches(r"(\pN)(\pN)(\pN)(\pN)", "ⅡⅢⅳⅥ");
    // print_matches("(aa)|(bb)", "bb"); 
    // print_matches("(>[^\n]+)?\n", ">name\nactg\n>name2\ngtca"); 
}

