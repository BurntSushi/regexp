use rand::Rng;
use std::str;

#[cfg(bench)]
mod bench;
#[cfg(not(bench), not(debug))]
mod test;

#[allow(dead_code)]
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

#[cfg(debug, test)]
mod debug {
    use super::super::Regexp;

    fn print_matches(re: &str, text: &str) {
        let r = Regexp::new(re).unwrap();
        let caps = r.captures(text).unwrap();
        for (i, s) in caps.iter().enumerate() {
            debug!("{} :: '{}'", caps.pos(i), s);
        }
        debug!("--------------------------");
    }

    #[test]
    fn debugging() {
        debug!("");
        // print_matches("(a?)((ab)?)(b?)", "ab"); 
        // print_matches("((a?)((ab)?))(b?)", "ab"); 
        // print_matches("(a*)*", "-"); 
        // print_matches("(a*|b)*", "-"); 
        // print_matches("(a+|b)*", "ab"); 
        // print_matches("(aba|a*b)*", "ababa"); 
        // print_matches("(a(b)?)+", "aba"); 
        // print_matches("(aa)|(bb)", "bb"); 
        // print_matches("(>[^\n]+)?\n", ">name\nactg\n>name2\ngtca"); 
        // print_matches("[[:lower:]]+", "`az{"); 
        // print_matches(r"(\pN)(\pN)(\pN)(\pN)", "ⅡⅢⅳⅥ"); 
        // print_matches(r"(a*)*", "ⅡⅢⅳⅥ"); 
        // debug!("{}", Regexp::new("abcd").unwrap().is_match("watabcd")); 
        // debug!("{}", Regexp::new("multiple words").unwrap().is_match("multiple words yeah")); 
        // print_matches(r"(A?AB?B)*", "AB"); 
        // print_matches(r"(A?AB?B)*", "AB"); 
        // print_matches(r"(a|bcdef|g|ab|c|d|e|efg|fg)*", "abcdefg"); 
        // debug!("{}", re!("z", "hello", "a")); 
        print_matches(r"b+", "bbbaaaa");
    }
}

