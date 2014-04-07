use rand::Rng;
use std::str;

#[cfg(large)]
mod large;
#[cfg(quickcheck)]
mod quick;
#[cfg(not(quickcheck), not(large))]
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
