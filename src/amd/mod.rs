#[macro_use]
mod macros;

use std::collections::{HashMap, HashSet};

use super::analyzer::{Analyzer, Stack};
use super::code::*;
use super::machine::MachineCode;
use super::model::Program;
use super::register::{Frame, Word};
use super::utils::*;

#[derive(Debug)]
pub struct AmdCompiler {
    machine_code: Vec<u8>,
    stack: Stack,
    allocs: HashMap<Word, u8>,
}

impl AmdCompiler {
    pub fn new() -> AmdCompiler {
        Self {
            machine_code: Vec::new(),
            stack: Stack::new(),
            allocs: HashMap::new(),
        }
    }

    pub fn emit(&mut self, v: Vec<u8>) {
        self.machine_code.extend_from_slice(&v[..]);
    }

    fn op_code(&mut self, op: &str, p: Proc, ry: u8) {
        match op {
            "mov" => {}
            "plus" => self.emit(amd! {addsd xmm(0), xmm(ry)}),
            "minus" => self.emit(amd! {subsd xmm(0), xmm(ry)}),
            "times" => self.emit(amd! {mulsd xmm(0), xmm(ry)}),
            "divide" => self.emit(amd! {divsd xmm(0), xmm(ry)}),
            "gt" => self.emit(amd! {cmpnlesd xmm(0), xmm(ry)}),
            "geq" => self.emit(amd! {cmpnltsd xmm(0), xmm(ry)}),
            "lt" => self.emit(amd! {cmpltsd xmm(0), xmm(ry)}),
            "leq" => self.emit(amd! {cmplesd xmm(0), xmm(ry)}),
            "eq" => self.emit(amd! {cmpeqsd xmm(0), xmm(ry)}),
            "neq" => self.emit(amd! {cmpneqsd xmm(0), xmm(ry)}),
            "and" => self.emit(amd! {andpd xmm(0), xmm(ry)}),
            "or" => self.emit(amd! {orpd xmm(0), xmm(ry)}),
            "xor" => self.emit(amd! {xorpd xmm(0), xmm(ry)}),
            "neg" => {
                self.emit(amd! {movsd xmm(1), qword ptr [rbp+8*Frame::MINUS_ZERO.0]});
                self.emit(amd! {xorpd xmm(0), xmm(1)});
            }
            _ => {
                self.emit(amd! {mov rax, qword ptr [rbx+8*p.0]});
                self.emit(amd! {call rax});
            }
        }
    }

    // xmm(2) == true ? xmm(0) : xmm(1)
    fn ifelse(&mut self) {
        self.emit(amd! {movapd xmm(3), xmm(2)});
        self.emit(amd! {andpd xmm(0), xmm(2)});
        self.emit(amd! {andnpd xmm(3), xmm(1)});
        self.emit(amd! {orpd xmm(0), xmm(3)});
    }

    fn load(&mut self, x: u8, r: Word, rename: bool) -> u8 {
        if let Some(s) = self.allocs.get(&r) {
            let s = *s;

            if s < 4 {
                if rename {
                    return s + 4;
                } else {
                    self.emit(amd! {movapd xmm(x), xmm(s+4)});
                    return x;
                }
            }
        }

        if r == Frame::ZERO {
            self.emit(amd! {xorpd xmm(x), xmm(x)});
        } else if r.is_temp() {
            let k = self.stack.pop(&r);
            self.emit(amd! {movsd xmm(x), qword ptr [rsp+8*k]});
        } else {
            self.emit(amd! {movsd xmm(x), qword ptr [rbp+8*r.0]});
        };

        x
    }

    fn save(&mut self, x: u8, r: Word) {
        if let Some(s) = self.allocs.get(&r) {
            let s = *s;

            if s < 4 {
                self.emit(amd! {movapd xmm(s+4), xmm(x)});
                return;
            }
        }

        if r.is_temp() {
            let k = self.stack.push(&r);
            self.emit(amd! {movsd qword ptr [rsp+8*k], xmm(x)});
        } else {
            self.emit(amd! {movsd qword ptr [rbp+8*r.0], xmm(x)});
        }
    }

    fn prologue(&mut self, n: usize) {
        self.emit(amd! {push rbp});
        self.emit(amd! {push rbx});
        self.emit(amd! {mov rbp, rdi});
        self.emit(amd! {mov rbx, rdx});
        self.emit(amd! {sub rsp, n});
    }

    fn epilogue(&mut self, n: usize) {
        self.emit(amd! {add rsp, n});
        self.emit(amd! {pop rbx});
        self.emit(amd! {pop rbp});
        self.emit(amd! {ret});
    }

    fn codegen(&mut self, prog: &Program, saveable: &HashSet<Word>) {
        let mut r = Frame::ZERO;

        for c in prog.code.iter() {
            match c {
                Instruction::Unary { p, x, dst, op } => {
                    if r != *x {
                        self.load(0, *x, false);
                    };
                    self.op_code(&op, *p, 0);
                    r = *dst;
                }
                Instruction::Binary { p, x, y, dst, op } => {
                    // commutative operators
                    let (x, y) = if (op == "plus" || op == "times") && *y == r {
                        (y, x)
                    } else {
                        (x, y)
                    };

                    let ry = if *y == r {
                        self.emit(amd! {movapd xmm(1), xmm(0)});
                        1
                    } else {
                        self.load(1, *y, true)
                    };

                    if *x != r {
                        self.load(0, *x, false);
                    }

                    self.op_code(&op, *p, ry);
                    r = *dst;
                }
                Instruction::IfElse { x1, x2, cond, dst } => {
                    if *cond == r {
                        self.emit(amd! {movapd xmm(2), xmm(0)});
                    } else {
                        self.load(2, *cond, false);
                    }

                    if *x2 == r {
                        self.emit(amd! {movapd xmm(1), xmm(0)});
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

impl Compiler<MachineCode> for AmdCompiler {
    fn compile(&mut self, prog: &Program) -> MachineCode {
        let analyzer = Analyzer::new(prog);
        let saveable = analyzer.find_saveable();

        self.allocs = analyzer.alloc_regs();

        self.codegen(prog, &saveable);
        self.machine_code.clear();
        let n = 8 * self.stack.capacity();
        self.prologue(n);
        self.codegen(prog, &saveable);
        self.epilogue(n);

        MachineCode::new(
            "x86_64",
            &self.machine_code.clone(),
            prog.virtual_table(),
            prog.frame.mem(),
        )
    }
}
