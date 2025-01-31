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
pub struct AmdCompiler {
    machine_code: Vec<u8>,
    buf: Vec<Option<Word>>,    
    stack: Stack,
    renamer: Renamer,
}

impl AmdCompiler {
    const XMM0: u8 = 0;
    const XMM1: u8 = 1;
    const XMM2: u8 = 2;
    const XMM3: u8 = 3;
    const XMM4: u8 = 4;
    const XMM5: u8 = 5;
    const XMM6: u8 = 6;
    const XMM7: u8 = 7;

    pub fn new() -> AmdCompiler {
        Self {
            machine_code: Vec::new(),
            buf: vec![None, None, None, None],
            stack: Stack::new(),
            renamer: Renamer::new(8),
        }
    }

    pub fn push_vec(&mut self, v: Vec<u8>) {
        self.machine_code.extend_from_slice(&v[..]);
    }
    
    fn n(&self, x: u8) -> u8 {
        self.renamer.get(x)
    }

    fn op_code(&mut self, op: &str, p: Proc) {                
        match op {
            "mov" => {}
            "plus" => self.push_vec(amd! {addsd xmm(self.n(0)), xmm(self.n(1))}),
            "minus" => self.push_vec(amd! {subsd xmm(self.n(0)), xmm(self.n(1))}),
            "times" => self.push_vec(amd! {mulsd xmm(self.n(0)), xmm(self.n(1))}),
            "divide" => self.push_vec(amd! {divsd xmm(self.n(0)), xmm(self.n(1))}),
            "gt" => self.push_vec(amd! {cmpnlesd xmm(self.n(0)), xmm(self.n(1))}),
            "geq" => self.push_vec(amd! {cmpnltsd xmm(self.n(0)), xmm(self.n(1))}),
            "lt" => self.push_vec(amd! {cmpltsd xmm(self.n(0)), xmm(self.n(1))}),
            "leq" => self.push_vec(amd! {cmplesd xmm(self.n(0)), xmm(self.n(1))}),
            "eq" => self.push_vec(amd! {cmpeqsd xmm(self.n(0)), xmm(self.n(1))}),
            "neq" => self.push_vec(amd! {cmpneqsd xmm(self.n(0)), xmm(self.n(1))}),
            "and" => self.push_vec(amd! {andpd xmm(self.n(0)), xmm(self.n(1))}),
            "or" => self.push_vec(amd! {orpd xmm(self.n(0)), xmm(self.n(1))}),
            "xor" => self.push_vec(amd! {xorpd xmm(self.n(0)), xmm(self.n(1))}),
            "neg" => {
                self.push_vec(amd! {movsd xmm(self.n(1)), qword ptr [rbp+8*Frame::MINUS_ZERO.0]});
                self.push_vec(amd! {xorpd xmm(self.n(0)), xmm(self.n(1))});
            }
            _ => {
                // self.dump_buffer();
                if self.n(0) != 0 {
                    self.push_vec(amd! {movapd xmm(0), xmm(self.n(0))});
                }
                self.renamer.reset();
                self.push_vec(amd! {mov rax, qword ptr [rbx+8*p.0]});
                self.push_vec(amd! {call rax});
            }
        }
    }

    // xmm(self.n(2)) == true ? xmm(self.n(0)) : xmm(self.n(1))
    fn ifelse(&mut self) {        
        self.push_vec(amd! {movapd xmm(self.n(3)), xmm(self.n(2))});        
        self.push_vec(amd! {andpd xmm(self.n(0)), xmm(self.n(2))});
        self.push_vec(amd! {andnpd xmm(self.n(3)), xmm(self.n(1))});
        self.push_vec(amd! {orpd xmm(self.n(0)), xmm(self.n(3))});
    }

    fn load_xmm_indirect(&mut self, x: u8, r: Word) {
        if r == Frame::ZERO {
            self.push_vec(amd! {xorpd xmm(self.n(x)), xmm(self.n(x))});
        } else if r.is_temp() {
            let k = self.stack.pop(&r);            
            self.push_vec(amd! {movsd xmm(self.n(x)), qword ptr [rsp+8*k]});
        } else {
            self.push_vec(amd! {movsd xmm(self.n(x)), qword ptr [rbp+8*r.0]});
        }
    }

    fn save_xmm_indirect(&mut self, x: u8, r: Word) {
        if r.is_temp() {
            let k = self.stack.push(&r);
            self.push_vec(amd! {movsd qword ptr [rsp+8*k], xmm(self.n(x))});
        } else {
            self.push_vec(amd! {movsd qword ptr [rbp+8*r.0], xmm(self.n(x))});
        }        
    }

    fn load_buffered(&mut self, x: u8, r: Word) {
        for (k, b) in self.buf.iter().enumerate() {
            if b.is_some_and(|s| s == r) {
                //self.push_vec(amd! {movapd xmm(x), xmm((4+k) as u8)});
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
                // self.push_vec(amd! {movapd xmm((4+k) as u8), xmm(x)});
                self.renamer.swap((4+k) as u8, x);
                self.buf[k] = Some(r);
                return;
            }
        }

        self.save_xmm_indirect(x, r);
    }

    fn prologue(&mut self, n: usize) {
        self.push_vec(amd! {push rbp});
        self.push_vec(amd! {push rbx});
        self.push_vec(amd! {mov rbp, rdi});
        self.push_vec(amd! {mov rbx, rdx});
        self.push_vec(amd! {sub rsp, n});
    }

    fn epilogue(&mut self, n: usize) {
        self.push_vec(amd! {add rsp, n});
        self.push_vec(amd! {pop rbx});
        self.push_vec(amd! {pop rbp});
        self.push_vec(amd! {ret});
    }
    
    fn codegen(&mut self, prog: &Program, saveable: &HashSet<Word>, bufferable: &HashSet<Word>) {
        let mut r = Frame::ZERO;
    
        for c in prog.code.iter() {
            match c {
                Instruction::Unary { p, x, dst, op } => {
                    if r != *x {
                        self.load_buffered(Self::XMM0, *x);
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
                        // self.push_vec(amd! {movapd xmm(self.n(1)), xmm(self.n(0))});
                        self.renamer.swap(1, 0);
                    } else {
                        self.load_buffered(Self::XMM1, *y);
                    }

                    if *x != r {
                        self.load_buffered(Self::XMM0, *x);
                    }

                    self.op_code(&op, *p);
                    r = *dst;
                }
                Instruction::IfElse { x1, x2, cond, dst } => {
                    if *cond == r {
                        // self.push_vec(amd! {movapd xmm(self.n(2)), xmm(self.n(0))});
                        self.renamer.swap(2, 0);
                    } else {
                        self.load_buffered(Self::XMM2, *cond);
                    }

                    if *x2 == r {
                        // self.push_vec(amd! {movapd xmm(self.n(1)), xmm(self.n(0))});
                        self.renamer.swap(1, 0);
                    } else {
                        self.load_buffered(Self::XMM1, *x2);
                    }

                    if *x1 != r {
                        self.load_buffered(Self::XMM0, *x1);
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
                self.save_xmm_indirect(Self::XMM0, r);
                r = Frame::ZERO;
            }

            // A bufferable register can be buffered without the
            // need for self.dump_buffer()
            if bufferable.contains(&r) {                
                self.save_buffered(Self::XMM0, r);
                r = Frame::ZERO;
            }

            // A saveable register can be saved directly or buffered
            // However, if it is buffered, self.dump_buffer() should be
            // uncommented in fn op_code
            if saveable.contains(&r) {
                self.save_xmm_indirect(Self::XMM0, r);
                r = Frame::ZERO;
            }
        }
    }
}

impl Compiler<MachineCode> for AmdCompiler {
    fn compile(&mut self, prog: &Program) -> MachineCode {                
        let analyzer = Analyzer::new(prog);
        let saveable = analyzer.find_saveable();
        let bufferable = analyzer.find_bufferable();
        
        self.codegen(prog, &saveable, &bufferable);             
        self.machine_code.clear();        
        let n = 8 * self.stack.capacity();
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



