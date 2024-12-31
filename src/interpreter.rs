use crate::code::*;
use crate::model::Program;
use crate::register::Reg;
use crate::utils::*;

#[derive(Debug)]
pub struct Interpreter {
}

impl Interpreter {    
    pub fn new() -> Interpreter {
        Self {            
        }
    }    
}

impl Compiler<ByteCode> for Interpreter {
    fn compile(&mut self, prog: &Program) -> ByteCode {
        ByteCode::new(prog.code.clone(), prog.virtual_table())
    }
}


#[derive(Debug)]
pub struct ByteCode {
    code:   Vec<Instruction>,
    vt:     Vec<BinaryFunc>,
}

impl ByteCode  {
    fn new(code: Vec<Instruction>, vt: Vec<BinaryFunc>) -> ByteCode {
        ByteCode {
            code,
            vt,
        }
    }
    
}

impl Compiled for ByteCode {
    fn run(&self, mem: &mut [f64]) {
        for c in self.code.iter()  {
            match c {
                Instruction::Num {..} => {},    // Num and Var do not generate any code 
                Instruction::Var {..} => {},    // They are mainly for debugging
                Instruction::Op {p, x, y, dst, ..} => { 
                    mem[dst.0] = self.vt[p.0](mem[x.0], mem[y.0]);
                }
            }
        }
    }
}



