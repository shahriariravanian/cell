#[macro_use]
mod macros;

use std::collections::{HashSet, HashMap};

use super::analyzer::{Analyzer, Stack};
use super::code::*;
use super::machine::MachineCode;
use super::model::Program;
use super::register::{Frame, Word};
use super::utils::*;

#[derive(Debug)]
pub struct ArmCompiler {
    machine_code: Vec<u8>,
    stack: Stack,
    allocs: HashMap<Word, u8>,
}

impl ArmCompiler {
    pub fn new() -> ArmCompiler {
        Self {
            machine_code: Vec::new(),
            stack: Stack::new(),
            allocs: HashMap::new(),
        }
    }

    pub fn emit(&mut self, w: u32) {
        self.machine_code.push(w as u8);
        self.machine_code.push((w >> 8) as u8);
        self.machine_code.push((w >> 16) as u8);
        self.machine_code.push((w >> 24) as u8);
    }    

    fn op_code(&mut self, op: &str, p: Proc) {
        match op {
            "mov" => {}
            "plus" => self.emit(arm! {fadd d(0), d(0), d(1)}),
            "minus" => self.emit(arm! {fsub d(0), d(0), d(1)}),
            "times" => self.emit(arm! {fmul d(0), d(0), d(1)}),
            "divide" => self.emit(arm! {fdiv d(0), d(0), d(1)}),
            "gt" => self.emit(arm! {fcmgt d(0), d(0), d(1)}),
            "geq" => self.emit(arm! {fcmge d(0), d(0), d(1)}),
            "lt" => self.emit(arm! {fcmlt d(0), d(0), d(1)}),
            "leq" => self.emit(arm! {fcmle d(0), d(0), d(1)}),
            "eq" => self.emit(arm! {fcmeq d(0), d(0), d(1)}),
            "and" => self.emit(arm! {and v(0).8b, v(0).8b, v(1).8b}),
            "or" => self.emit(arm! {orr v(0).8b, v(0).8b, v(1).8b}),
            "xor" => self.emit(arm! {eor v(0).8b, v(0).8b, v(1).8b}),
            "neg" => self.emit(arm! {fneg d(0), d(0)}),
            "root" => self.emit(arm! {fsqrt d(0), d(0)}),
            "neq" => {
                self.emit(arm! {fcmeq d(0), d(0), d(1)});
                self.emit(arm! {not v(0).8b, v(0).8b});
            }
            _ => {
                self.emit(arm! {ldr x(0), [x(20), #8*p.0]});
                self.emit(arm! {blr x(0)});
            }
        }
    }

    // d2 == true ? d0 : d1
    fn ifelse(&mut self) {
        self.emit(arm! {bsl v(2).8b, v(0).8b, v(1).8b});
        self.emit(arm! {fmov d(0), d(2)});        
    }

    fn load(&mut self, x: u8, r: Word, rename: bool) -> u8 {
        if let Some(s) = self.allocs.get(&r) {
            let s = *s;
            
            if s < 4 {
                if rename {
                    return s + 4;
                } else {
                    self.emit(arm! {fmov d(x), d(s+4)});
                    return x;
                }
            }
        }
        
        if r == Frame::ZERO {
            self.emit(arm! {fmov d(x), #0.0});
        } else if r == Frame::ONE {
            self.emit(arm! {fmov d(x), #1.0});
        } else if r == Frame::MINUS_ONE {
            self.emit(arm! {fmov d(x), #-1.0});
        } else if r.is_temp() {
            let k = self.stack.pop(&r);            
            self.emit(arm! {ldr d(x), [sp, #8*k]});
        } else {
            self.emit(arm! {ldr d(x), [x(19), #8*r.0]});
        };
        
        x
    }

    fn save(&mut self, x: u8, r: Word) {
        if let Some(s) = self.allocs.get(&r) {
            let s = *s;
            
            if s < 4 {
                self.emit(arm! {fmov d(s+4), d(x)});
                return;
            }
        }
    
        if r.is_temp() {
            let k = self.stack.push(&r);
            self.emit(arm! {str d(x), [sp, #8*k]});
        } else {
            self.emit(arm! {str d(x), [x(19), #8*r.0]});
        }
    }    

    fn prologue(&mut self, n: usize) {
        self.emit(arm! {sub sp, sp, #n+32});
        self.emit(arm! {str lr, [sp, #n]});
        self.emit(arm! {stp x(19), x(20), [sp, #n+16]});
        self.emit(arm! {mov x(19), x(0)});
        self.emit(arm! {mov x(20), x(2)});        
    }

    fn epilogue(&mut self, n: usize) {
        self.emit(arm! {ldp x(19), x(20), [sp, #n+16]});
        self.emit(arm! {ldr lr, [sp, #n]});
        self.emit(arm! {add sp, sp, #n+32});
        self.emit(arm! {ret});
    }
    
    fn codegen(&mut self, prog: &Program, saveable: &HashSet<Word>) {
        let mut r = Frame::ZERO;
        
        for c in prog.code.iter() {
            match c {
                Instruction::Unary { p, x, dst, op } => {
                    if r != *x {
                        self.load(0, *x, false);
                    };
                    self.op_code(&op, *p);
                    r = *dst;
                }
                Instruction::Binary { p, x, y, dst, op } => {
                    // commutative operators
                    let (x, y) = if (op == "plus" || op == "times") && *y == r {
                        (y, x)
                    } else {
                        (x, y)
                    };

                    if *y == r {
                        self.emit(arm! {fmov d(1), d(0)});                        
                    } else {
                        self.load(1, *y, false);
                    }

                    if *x != r {
                        self.load(0, *x, false);
                    }

                    self.op_code(&op, *p);
                    r = *dst;
                }
                Instruction::IfElse { x1, x2, cond, dst } => {
                    if *cond == r {
                        self.emit(arm! {fmov d(2), d(0)});                        
                    } else {
                        self.load(2, *cond, false);
                    }

                    if *x2 == r {
                        self.emit(arm! {fmov d(1), d(0)});                        
                    } else {
                        self.load(1, *x2, false);
                    }

                    if *x1 != r {
                        self.load(0, *x1, false);
                    }

                    self.ifelse();
                    r = *dst;
                }
                _ => {
                    continue;
                }
            }

            if prog.frame.is_diff(&r) || saveable.contains(&r) {
                self.save(0, r);
                r = Frame::ZERO;
            }
        }    
    }
}

impl Compiler<MachineCode> for ArmCompiler {
    fn compile(&mut self, prog: &Program) -> MachineCode {
        let analyzer = Analyzer::new(prog);
        let saveable = analyzer.find_saveable();
        
        self.allocs = analyzer.alloc_regs();
        
        self.codegen(prog, &saveable);             
        self.machine_code.clear();        
        let n = 8 * ((self.stack.capacity() + 1) & 0xfff7);
        self.prologue(n);
        self.codegen(prog, &saveable);
        self.epilogue(n);

        MachineCode::new(
            "aarch64",
            &self.machine_code.clone(),
            prog.virtual_table(),
            prog.frame.mem(),
        )
    }
}
