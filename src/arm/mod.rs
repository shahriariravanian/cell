#[macro_use]
mod macros;

use std::collections::HashSet;

use super::analyzer::{Analyzer, Renamer, Stack};
use super::code::*;
use super::machine::MachineCode;
use super::model::Program;
use super::register::{Frame, Word};
use super::utils::*;

#[derive(Debug)]
pub struct ArmCompiler {
    machine_code: Vec<u8>,
    buf: Vec<Option<Word>>,    
    stack: Stack,
    renamer: Renamer,
}

impl ArmCompiler {
    const D0: u8 = 0;
    const D1: u8 = 1;
    const D2: u8 = 2;
    const D3: u8 = 3;
    const D4: u8 = 4;
    const D5: u8 = 5;
    const D6: u8 = 6;
    const D7: u8 = 7;

    pub fn new() -> ArmCompiler {
        Self {
            machine_code: Vec::new(),
            buf: vec![None, None, None, None],
            stack: Stack::new(),
            renamer: Renamer::new(8),
        }
    }

    pub fn push_u32(&mut self, w: u32) {
        self.machine_code.push(w as u8);
        self.machine_code.push((w >> 8) as u8);
        self.machine_code.push((w >> 16) as u8);
        self.machine_code.push((w >> 24) as u8);
    }
    
    fn n(&self, x: u8) -> u8 {
        self.renamer.get(x)
    }

    fn op_code(&mut self, op: &str, p: Proc) {
        match op {
            "mov" => {}
            "plus" => self.push_u32(arm! {fadd d(self.n(0)), d(self.n(0)), d(self.n(1))}),
            "minus" => self.push_u32(arm! {fsub d(self.n(0)), d(self.n(0)), d(self.n(1))}),
            "times" => self.push_u32(arm! {fmul d(self.n(0)), d(self.n(0)), d(self.n(1))}),
            "divide" => self.push_u32(arm! {fdiv d(self.n(0)), d(self.n(0)), d(self.n(1))}),
            "gt" => self.push_u32(arm! {fcmgt d(self.n(0)), d(self.n(0)), d(self.n(1))}),
            "geq" => self.push_u32(arm! {fcmge d(self.n(0)), d(self.n(0)), d(self.n(1))}),
            "lt" => self.push_u32(arm! {fcmlt d(self.n(0)), d(self.n(0)), d(self.n(1))}),
            "leq" => self.push_u32(arm! {fcmle d(self.n(0)), d(self.n(0)), d(self.n(1))}),
            "eq" => self.push_u32(arm! {fcmeq d(self.n(0)), d(self.n(0)), d(self.n(1))}),
            "and" => self.push_u32(arm! {and v(self.n(0)).8b, v(self.n(0)).8b, v(self.n(1)).8b}),
            "or" => self.push_u32(arm! {orr v(self.n(0)).8b, v(self.n(0)).8b, v(self.n(1)).8b}),
            "xor" => self.push_u32(arm! {eor v(self.n(0)).8b, v(self.n(0)).8b, v(self.n(1)).8b}),
            "neg" => self.push_u32(arm! {fneg d(self.n(0)), d(self.n(0))}),
            "root" => self.push_u32(arm! {fsqrt d(self.n(0)), d(self.n(0))}),
            "neq" => {
                self.push_u32(arm! {fcmeq d(self.n(0)), d(self.n(0)), d(self.n(1))});
                self.push_u32(arm! {not v(self.n(0)).8b, v(self.n(0)).8b});
            }
            _ => {
                // self.dump_buffer();
                 if self.n(0) != 0 {
                    self.push_u32(arm! {fmov d(0), d(self.n(0))});
                }
                self.renamer.reset();               
                self.push_u32(arm! {ldr x(0), [x(20), #8*p.0]});
                self.push_u32(arm! {blr x(0)});
            }
        }
    }

    // d2 == true ? d0 : d1
    fn ifelse(&mut self) {
        self.push_u32(arm! {and v(self.n(2)).8b, v(self.n(0)).8b, v(self.n(1)).8b});
        self.renamer.swap(0, 2);
        // self.push_u32(arm! {and v(self.n(0)).8b, v(self.n(0)).8b, v(self.n(2)).8b});
        // self.push_u32(arm! {not v(self.n(2)).8b, v(self.n(2)).8b});
        // self.push_u32(arm! {and v(self.n(1)).8b, v(self.n(1)).8b, v(self.n(2)).8b});
        // self.push_u32(arm! {orr v(self.n(0)).8b, v(self.n(0)).8b, v(self.n(1)).8b});
    }

    fn load_xmm_indirect(&mut self, x: u8, r: Word) {
        if r == Frame::ZERO {
            self.push_u32(arm! {fmov d(self.n(x)), #0.0});
        } else if r == Frame::ONE {
            self.push_u32(arm! {fmov d(self.n(x)), #1.0});
        } else if r == Frame::MINUS_ONE {
            self.push_u32(arm! {fmov d(self.n(x)), #-1.0});
        } else if r.is_temp() {
            let k = self.stack.pop(&r);            
            self.push_u32(arm! {ldr d(self.n(x)), [sp, #8*k]});
        } else {
            self.push_u32(arm! {ldr d(self.n(x)), [x(19), #8*r.0]});
        }
    }

    fn save_xmm_indirect(&mut self, x: u8, r: Word) {
        if r.is_temp() {
            let k = self.stack.push(&r);
            self.push_u32(arm! {str d(self.n(x)), [sp, #8*k]});
        } else {
            self.push_u32(arm! {str d(self.n(x)), [x(19), #8*r.0]});
        }
    }
    
    fn load_buffered(&mut self, x: u8, r: Word) {
        for (k, b) in self.buf.iter().enumerate() {
            if b.is_some_and(|s| s == r) {                
                // self.push_u32(arm! {fmov d(self.n(x)), d(self.n((4+k) as u8))});
                self.renamer.swap(x, (4+k) as u8);
                self.buf[k] = None;
                return;
            }        
        }
        
        self.load_xmm_indirect(x, r);
    }

    fn save_buffered(&mut self, x: u8, r: Word) {        
        for (k, b) in self.buf.iter().enumerate() {
            if b.is_none() {
                // self.push_u32(arm! {fmov d(self.n((4+k) as u8)), d(self.n(x))});
                self.renamer.swap((4+k) as u8, x);
                self.buf[k] = Some(r);
                return;
            }
        }

        self.save_xmm_indirect(x, r);
    }

    fn prologue(&mut self, n: usize) {
        self.push_u32(arm! {sub sp, sp, #32});
        self.push_u32(arm! {str lr, [sp, #0]});
        self.push_u32(arm! {stp x(19), x(20), [sp, #16]});
        self.push_u32(arm! {mov x(19), x(0)});
        self.push_u32(arm! {mov x(20), x(2)});
        self.push_u32(arm! {sub sp, sp, #n});
    }

    fn epilogue(&mut self, n: usize) {
        self.push_u32(arm! {add sp, sp, #n});
        self.push_u32(arm! {ldp x(19), x(20), [sp, #16]});
        self.push_u32(arm! {ldr lr, [sp, #0]});
        self.push_u32(arm! {add sp, sp, #32});
        self.push_u32(arm! {ret});
    }
    
    fn codegen(&mut self, prog: &Program, saveable: &HashSet<Word>, bufferable: &HashSet<Word>) {
        let mut r = Frame::ZERO;
        
        for c in prog.code.iter() {
            match c {
                Instruction::Unary { p, x, dst, op } => {
                    if r != *x {
                        self.load_buffered(0, *x);
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
                        // self.push_u32(arm! {fmov d(self.n(1)), d(self.n(0))});
                        self.renamer.swap(1, 0);
                    } else {
                        self.load_buffered(1, *y);
                    }

                    if *x != r {
                        self.load_buffered(0, *x);
                    }

                    self.op_code(&op, *p);
                    r = *dst;
                }
                Instruction::IfElse { x1, x2, cond, dst } => {
                    if *cond == r {
                        // self.push_u32(arm! {fmov d(self.n(2)), d(self.n(0))});
                        self.renamer.swap(2, 0);
                    } else {
                        self.load_buffered(2, *cond);
                    }

                    if *x2 == r {
                        // self.push_u32(arm! {fmov d(self.n(1)), d(self.n(0))});
                        self.renamer.swap(1, 0);
                    } else {
                        self.load_buffered(1, *x2);
                    }

                    if *x1 != r {
                        self.load_buffered(0, *x1);
                    }

                    self.ifelse();
                    r = *dst;
                }
                _ => {
                    continue;
                }
            }

            // A diff register should be saved, cannot be buffered
            if prog.frame.is_diff(&r) {
                self.save_xmm_indirect(Self::D0, r);
                r = Frame::ZERO;
            }

            // A bufferable register can be buffered without the
            // need for self.dump_buffer()
            if bufferable.contains(&r) {
                self.save_buffered(Self::D0, r);
                r = Frame::ZERO;
            }

            // A saveable register can be saved directly or buffered
            // However, if it is buffered, self.dump_buffer() should be
            // uncommented in fn op_code
            if saveable.contains(&r) {
                self.save_xmm_indirect(Self::D0, r);
                r = Frame::ZERO;
            }
        }    
    }
}

impl Compiler<MachineCode> for ArmCompiler {
    fn compile(&mut self, prog: &Program) -> MachineCode {
        let analyzer = Analyzer::new(prog);
        let saveable = analyzer.find_saveable();
        let bufferable = analyzer.find_bufferable();
        
        self.codegen(prog, &saveable, &bufferable);             
        self.machine_code.clear();        
        let n = 8 * ((self.stack.capacity() + 1) & 0xfff7);
        self.prologue(n);
        self.codegen(prog, &saveable, &bufferable);
        self.epilogue(n);

        MachineCode::new(
            &self.machine_code.clone(),
            prog.virtual_table(),
            prog.frame.mem(),
        )
    }
}
