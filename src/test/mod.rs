use rand::Rng;
use std::str;

#[cfg(not(debug), large)]
mod large;
#[cfg(not(debug), quickcheck)]
mod quick;
#[cfg(not(debug), not(quickcheck), not(large))]
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
        print_matches(r"(\pN)(\pN)(\pN)(\pN)", "ⅡⅢⅳⅥ");
    }
}

