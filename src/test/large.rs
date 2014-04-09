use rand::task_rng;
use super::super::compile::Program;
use super::super::parse::parse;
use super::super::quote;
use super::gen_regex_str;

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
    let _ = Program::new(parse(s).unwrap());
}
