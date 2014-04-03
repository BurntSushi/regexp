use super::parse;
use super::parse::{Literal, Dot, Begin, End, Capture, Cat, Alt, Rep};
use super::parse::{ZeroOne, ZeroMore, OneMore, Greedy, Ungreedy};

type InstIdx = uint;

#[deriving(Show)]
pub enum Inst {
    // When a Match instruction is executed, the current thread is successful.
    Match,

    // The Char instruction matches a literal character.
    // If the bool is true, then the match is done case insensitively.
    Char(char, bool),

    // Matches any character except new lines.
    // If the bool is true, then new lines are matched.
    Any(bool),

    // Matches the beginning of the string, consumes no characters.
    // If the bool is true, then it also matches when the preceding character
    // is a new line.
    EmptyBegin(bool),

    // Matches the end of the string, consumes no characters.
    // If the bool is true, then it also matches when the proceeding character
    // is a new line.
    EmptyEnd(bool),

    // Saves the current position in the input string to the Nth save slot.
    Save(uint),

    // Jumps to the instruction at the index given.
    Jump(InstIdx),

    // Jumps to the instruction at the first index given. If that leads to
    // a failing state, then the instruction at the second index given is
    // tried.
    Split(InstIdx, InstIdx),
}

pub fn compile(ast: ~parse::Ast) -> Vec<Inst> {
    let mut c = Compiler { insts: Vec::with_capacity(100), };
    c.insts.push(Save(0));
    c.compile(ast);
    c.insts.push(Save(1));
    c.insts.push(Match);
    c.insts
}

struct Compiler {
    insts: Vec<Inst>,
}

impl Compiler {
    fn compile(&mut self, ast: ~parse::Ast) {
        match ast {
            ~Literal(c, casei) => self.push(Char(c, casei)),
            ~Dot(nl) => self.push(Any(nl)),
            ~Begin(multi) => self.push(EmptyBegin(multi)),
            ~End(multi) => self.push(EmptyEnd(multi)),
            ~Capture(cap, x) => {
                self.push(Save(2 * cap));
                self.compile(x);
                self.push(Save(2 * cap + 1));
            }
            ~Cat(x, y) => { self.compile(x); self.compile(y); }
            ~Alt(x, y) => {
                let split = self.empty_split(); // push: split 0, 0
                let j1 = self.insts.len();
                self.compile(x);                // push: insts for x
                let jmp = self.empty_jump();    // push: jmp 0
                let j2 = self.insts.len();
                self.compile(y);                // push: insts for y
                let j3 = self.insts.len();

                self.set_split(split, j1, j2);  // split 0, 0 -> split j1, j2
                self.set_jump(jmp, j3);         // jmp 0      -> jmp j3
            }
            ~Rep(x, ZeroOne, g) => {
                let split = self.empty_split();
                let j1 = self.insts.len();
                self.compile(x);
                let j2 = self.insts.len();

                if g.is_greedy() {
                    self.set_split(split, j1, j2);
                } else {
                    self.set_split(split, j2, j1);
                }
            }
            ~Rep(x, ZeroMore, g) => {
                let j1 = self.insts.len();
                let split = self.empty_split();
                let j2 = self.insts.len();
                self.compile(x);
                let jmp = self.empty_jump();
                let j3 = self.insts.len();

                self.set_jump(jmp, j1);
                if g.is_greedy() {
                    self.set_split(split, j2, j3);
                } else {
                    self.set_split(split, j3, j2);
                }
            }
            ~Rep(x, OneMore, g) => {
                let j1 = self.insts.len();
                self.compile(x);
                let split = self.empty_split();
                let j2 = self.insts.len();

                if g.is_greedy() {
                    self.set_split(split, j1, j2);
                } else {
                    self.set_split(split, j2, j1);
                }
            }
        }
    }

    fn push(&mut self, x: Inst) {
        self.insts.push(x)
    }

    fn empty_split(&mut self) -> InstIdx {
        self.insts.push(Split(0, 0));
        self.insts.len() - 1
    }

    fn set_split(&mut self, i: InstIdx, pc1: InstIdx, pc2: InstIdx) {
        let split = self.insts.get_mut(i);
        match *split {
            Split(_, _) => *split = Split(pc1, pc2),
            _ => fail!("BUG: Invalid split index."),
        }
    }

    fn empty_jump(&mut self) -> InstIdx {
        self.insts.push(Jump(0));
        self.insts.len() - 1
    }

    fn set_jump(&mut self, i: InstIdx, pc: InstIdx) {
        let jmp = self.insts.get_mut(i);
        match *jmp {
            Jump(_) => *jmp = Jump(pc),
            _ => fail!("BUG: Invalid jump index."),
        }
    }
}

#[cfg(test)]
mod test {
    use super::super::parse;

    #[test]
    #[ignore]
    fn simple() {
        let re = match parse::parse("and") {
            Err(err) => fail!("Parse error: {}", err),
            Ok(re) => re,
        };
        debug!("{}", super::compile(re));
    }
}
