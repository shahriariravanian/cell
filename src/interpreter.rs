use crate::code::*;
use crate::model::Program;
use crate::utils::*;

#[derive(Debug)]
pub struct Interpreter {}

impl Interpreter {
    pub fn new() -> Interpreter {
        Self {}
    }
}

impl Compiler<ByteCode> for Interpreter {
    fn compile(&mut self, prog: &Program) -> ByteCode {
        ByteCode::new(prog.code.clone(), prog.virtual_table(), prog.frame.mem())
    }
}

#[derive(Debug)]
pub struct ByteCode {
    code: Vec<Instruction>,
    vt: Vec<BinaryFunc>,
    _mem: Vec<f64>,
}

impl ByteCode {
    fn new(code: Vec<Instruction>, vt: Vec<BinaryFunc>, _mem: Vec<f64>) -> ByteCode {
        ByteCode { code, vt, _mem }
    }
}

impl Compiled for ByteCode {
    fn run(&mut self) {
        for c in self.code.iter() {
            match c {
                Instruction::Nop {} => {}
                Instruction::Num { .. } => {} // Num and Var do not generate any code
                Instruction::Var { .. } => {} // They are mainly for debugging
                Instruction::Op { p, x, y, dst, .. } => {
                    self._mem[dst.0] = self.vt[p.0](self._mem[x.0], self._mem[y.0]);
                }
            }
        }
    }

    #[inline]
    fn mem(&self) -> &[f64] {
        &self._mem[..]
    }

    #[inline]
    fn mem_mut(&mut self) -> &mut [f64] {
        &mut self._mem[..]
    }
}
