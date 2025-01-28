use memmap2::{Mmap, MmapOptions};
use rand::distributions::{Alphanumeric, DistString};
use std::collections::HashSet;
use std::fs;
use std::io::Write;

#[macro_use]
mod macros;

use super::analyzer::Analyzer;
use super::code::*;
use super::machine::MachineCode;
use super::model::Program;
use super::register::{Frame, Word};
use super::utils::*;


#[derive(Debug)]
pub struct AmdCompiler {
    machine_code: Vec<u8>,
    optimize: bool,
    x4: Option<Word>,
    x5: Option<Word>,
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

    pub fn new(optimize: bool) -> AmdCompiler {
        Self {
            machine_code: Vec::new(),
            x4: None,
            x5: None,
            optimize,
        }
    }
    
    pub fn push_vec(&mut self, v: Vec<u8>) {
        self.machine_code.extend_from_slice(&v[..]);
    }

    fn op_code(&mut self, op: &str, p: Proc) {
        match op {
            "mov" => {}
            "plus" => self.push_vec(amd!{addsd xmm(0), xmm(1)}),
            "minus" => self.push_vec(amd!{subsd xmm(0), xmm(1)}),
            "times" => self.push_vec(amd!{mulsd xmm(0), xmm(1)}),
            "divide" => self.push_vec(amd!{divsd xmm(0), xmm(1)}),
            "gt" => self.push_vec(amd!{cmpnlesd xmm(0), xmm(1)}),
            "geq" => self.push_vec(amd!{cmpnltsd xmm(0), xmm(1)}),
            "lt" => self.push_vec(amd!{cmpltsd xmm(0), xmm(1)}),
            "leq" => self.push_vec(amd!{cmplesd xmm(0), xmm(1)}),
            "eq" => self.push_vec(amd!{cmpeqsd xmm(0), xmm(1)}),
            "neq" => self.push_vec(amd!{cmpneqsd xmm(0), xmm(1)}),
            "and" => self.push_vec(amd!{andpd xmm(0), xmm(1)}),
            "or" => self.push_vec(amd!{orpd xmm(0), xmm(1)}),
            "xor" => self.push_vec(amd!{xorpd xmm(0), xmm(1)}),
            "neg" => {
                self.push_vec(amd!{movsd xmm(1), qword ptr [rbp+8*Frame::MINUS_ZERO.0]});
                self.push_vec(amd!{xorpd xmm(0), xmm(1)});
            }
            _ => {
                if !self.optimize {
                    self.dump_buffer();
                }
                self.push_vec(amd!{mov rax, qword ptr [rbx+8*p.0]});
                self.push_vec(amd!{call rax});
            }
        }
    }

    // xmm(2) == true ? xmm(0) : xmm(1)
    fn ifelse(&mut self) {
        self.push_vec(amd!{movapd xmm(3), xmm(2)});
        self.push_vec(amd!{andpd xmm(0), xmm(2)});
        self.push_vec(amd!{andnpd xmm(3), xmm(1)});
        self.push_vec(amd!{orpd xmm(0), xmm(3)});
    }

    fn load_xmm_indirect(&mut self, x: u8, r: Word) {
        if r == Frame::ZERO {
            self.push_vec(amd!{xorpd xmm(x), xmm(x)});
        } else {
            self.push_vec(amd!{movsd xmm(x), qword ptr [rbp+8*r.0]});
        }
    }

    fn save_xmm_indirect(&mut self, x: u8, r: Word) {
        if r.0 > 2 {
            self.push_vec(amd!{movsd qword ptr [rbp+8*r.0], xmm(x)});
        }
    }

    fn load_buffered(&mut self, x: u8, r: Word) {
        if self.x4.is_some_and(|s| s == r) {
            self.push_vec(amd!{movapd xmm(x), xmm(4)});
            self.x4 = None;
            return;
        }

        if self.x5.is_some_and(|s| s == r) {
            self.push_vec(amd!{movapd xmm(x), xmm(5)});
            self.x5 = None;
            return;
        }

        self.load_xmm_indirect(x, r);
    }

    fn save_buffered(&mut self, x: u8, r: Word) {
        if self.x4.is_none() {
            self.push_vec(amd!{movapd xmm(4), xmm(x)});
            self.x4 = Some(r);
            return;
        }

        if self.x5.is_none() {
            self.push_vec(amd!{movapd xmm(5), xmm(x)});
            self.x5 = Some(r);
            return;
        }

        self.save_xmm_indirect(x, r);
    }

    fn dump_buffer(&mut self) {
        if let Some(s) = self.x4 {
            self.save_xmm_indirect(Self::XMM4, s);
            self.x4 = None;
        }

        if let Some(s) = self.x5 {
            self.save_xmm_indirect(Self::XMM5, s);
            self.x5 = None;
        }
    }
    
    fn prologue(&mut self) {
        self.push_vec(amd!{push rbp});
        self.push_vec(amd!{push rbx});
        self.push_vec(amd!{mov rbp, rdi});
        self.push_vec(amd!{mov rbx, rdx});
    }
    
    fn epilogue(&mut self) {
        self.push_vec(amd!{pop rbx});
        self.push_vec(amd!{pop rbp});
        self.push_vec(amd!{ret});
    }
}

impl Compiler<MachineCode> for AmdCompiler {
    fn compile(&mut self, prog: &Program) -> MachineCode {
        self.prologue();
             
        let mut r = Frame::ZERO;

        let analyzer = Analyzer::new(prog);
        let saveables = analyzer.find_saveables();

        let bufferable: HashSet<Word> = if self.optimize {
            analyzer.find_bufferable()
        } else {
            HashSet::new()
        };

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
                        self.push_vec(amd!{movapd xmm(1), xmm(0)});
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
                        self.push_vec(amd!{movapd xmm(2), xmm(0)});
                    } else {
                        self.load_buffered(Self::XMM2, *cond);
                    }

                    if *x2 == r {
                        self.push_vec(amd!{movapd xmm(1), xmm(0)});
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
            if self.optimize && bufferable.contains(&r) {
                self.save_buffered(Self::XMM0, r);
                r = Frame::ZERO;
            }

            // A saveable register can be saved directly or buffered
            // However, if it is buffered, self.dump_buffer() should be
            // uncommented in fn op_code
            if saveables.contains(&r) {
                if self.optimize {
                    self.save_xmm_indirect(Self::XMM0, r);
                } else {
                    self.save_buffered(Self::XMM0, r);
                }
                r = Frame::ZERO;
            }
        }
        
        self.epilogue();        

        // println!("{}", &self.assembler);

        MachineCode::new(            
            &self.machine_code.clone(),
            prog.virtual_table(),
            prog.frame.mem(),
        )
    }
}
