use super::parse;
use super::parse::{Literal, Cat, Alt, Rep};
use super::parse::{ZeroOne, ZeroMore, OneMore, Greedy, Ungreedy};

#[deriving(Show)]
enum Inst {
    Char(char),
    Match,
    Jump(uint),
    Split(uint, uint),
}

fn compile(ast: ~parse::Ast) -> Vec<Inst> {
    let mut c = Compiler { insts: Vec::with_capacity(100), };
    c.compile(ast);
    c.insts.push(Match);
    c.insts
}

struct Compiler {
    insts: Vec<Inst>,
}

impl Compiler {
    fn compile(&mut self, ast: ~parse::Ast) {
        match ast {
            ~Literal(c) => self.push(Char(c)),
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

    fn empty_split(&mut self) -> uint {
        self.insts.push(Split(0, 0));
        self.insts.len() - 1
    }

    fn set_split(&mut self, i: uint, pc1: uint, pc2: uint) {
        let split = self.insts.get_mut(i);
        match *split {
            Split(_, _) => *split = Split(pc1, pc2),
            _ => fail!("BUG: Invalid split index."),
        }
    }

    fn empty_jump(&mut self) -> uint {
        self.insts.push(Jump(0));
        self.insts.len() - 1
    }

    fn set_jump(&mut self, i: uint, pc: uint) {
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
    fn simple() {
        let re = match parse::parse("a+b+") {
            Err(err) => fail!("Parse error: {}", err),
            Ok(re) => re,
        };
        debug!("{}", super::compile(re));
    }
}
