use memmap2::{Mmap, MmapOptions};
use rand::distributions::{Alphanumeric, DistString};
use std::collections::HashSet;
use std::fs;
use std::io::Write;

use crate::assembler::Assembler;
use crate::code::*;
use crate::model::Program;
use crate::register::Word;
use crate::utils::*;

#[derive(Debug)]
pub struct NativeCompiler {
    assembler: Assembler,
    optimize: bool,
    x4: Option<Word>,
    x5: Option<Word>,
}

pub enum Linear {
    Producer(Word),
    Consumer(Word),
    Caller(String),
}

impl NativeCompiler {
    const XMM0: u8 = 0;
    const XMM1: u8 = 1;
    const XMM2: u8 = 2;
    const XMM3: u8 = 3;
    const XMM4: u8 = 4;
    const XMM5: u8 = 5;
    const XMM6: u8 = 6;
    const XMM7: u8 = 7;

    pub fn new(optimize: bool) -> NativeCompiler {
        Self {
            assembler: Assembler::new(),
            x4: None,
            x5: None,
            optimize,
        }
    }

    fn push(&mut self, s: &str) {
        self.assembler.push(s);
    }

    fn op_code(&mut self, op: &str, p: Proc) {
        match op {
            "mov" => {}
            "plus" => self.push("addsd xmm0, xmm1"),
            "minus" => self.push("subsd xmm0, xmm1"),
            "times" => self.push("mulsd xmm0, xmm1"),
            "divide" => self.push("divsd xmm0, xmm1"),
            "gt" => self.push("cmpnlesd xmm0, xmm1"),
            "geq" => self.push("cmpnltsd xmm0, xmm1"),
            "lt" => self.push("cmpltsd xmm0, xmm1"),
            "leq" => self.push("cmplesd xmm0, xmm1"),
            "eq" => self.push("cmpeqsd xmm0, xmm1"),
            "neq" => self.push("cmpneqsd xmm0, xmm1"),
            "and" => self.push("andpd xmm0, xmm1"),
            "or" => self.push("orpd xmm0, xmm1"),
            "xor" => self.push("xorpd xmm0, xmm1"),
            "neg" => {
                self.push(format!("movsd xmm1, qword ptr [rbp+0x{:x}]", 8 * Word(3).0).as_str());
                self.push("xorpd xmm0, xmm1")
            }
            _ => {
                if !self.optimize {
                    self.dump_buffer();
                }
                self.push(format!("mov rax, qword ptr [rbx+0x{:x}]", 8 * p.0).as_str());
                self.push("call rax");
            }
        }
    }

    // xmm2 == true ? xmm0 : xmm1
    fn ifelse(&mut self) {
        self.push("movsd xmm3, xmm2");
        self.push("andpd xmm0, xmm2");
        self.push("andnpd xmm3, xmm1");
        self.push("addsd xmm0, xmm3");
    }

    fn load_xmm_indirect(&mut self, x: u8, r: Word) {
        if r == Word(0) {
            self.push(format!("xorpd xmm{}, xmm{}", x, x).as_str());
        } else {
            self.push(format!("movsd xmm{}, qword ptr [rbp+0x{:x}]", x, 8 * r.0).as_str());
        }
    }

    fn save_xmm_indirect(&mut self, x: u8, r: Word) {
        if r.0 > 2 {
            self.push(format!("movsd qword ptr [rbp+0x{:x}], xmm{}", 8 * r.0, x).as_str());
        }
    }

    fn linearize(&self, prog: &Program) -> Vec<Linear> {
        let mut linear: Vec<Linear> = Vec::new();

        for c in prog.code.iter() {
            match c {
                Instruction::Unary { op, x, dst, .. } => {
                    linear.push(Linear::Consumer(*x));
                    linear.push(Linear::Caller(op.clone()));
                    linear.push(Linear::Producer(*dst));
                }
                Instruction::Binary { op, x, y, dst, .. } => {
                    linear.push(Linear::Consumer(*x));
                    linear.push(Linear::Consumer(*y));
                    linear.push(Linear::Caller(op.clone()));
                    linear.push(Linear::Producer(*dst));
                }
                Instruction::IfElse { x1, x2, cond, dst } => {
                    linear.push(Linear::Consumer(*x1));
                    linear.push(Linear::Consumer(*x2));
                    linear.push(Linear::Consumer(*cond));
                    linear.push(Linear::Caller("select".to_string()));
                    linear.push(Linear::Producer(*dst));
                }
                _ => {}
            }
        }
        linear
    }

    /*
        A saveable register is produced but is not consumed immediately
        In other words, it cannot be coalesced over consecuative instructions
    */
    fn find_saveables(&self, linear: &Vec<Linear>) -> HashSet<Word> {
        let mut candidates: Vec<Word> = Vec::new();
        let mut saveables: HashSet<Word> = HashSet::new();

        for l in linear.iter() {
            match l {
                Linear::Producer(p) => {
                    candidates.push(*p);
                }
                Linear::Consumer(c) => {
                    let r = candidates.pop();
                    if candidates.contains(c) {
                        saveables.insert(*c);
                    };
                    if r.is_some() {
                        candidates.push(r.unwrap());
                    };
                }
                Linear::Caller(_) => {}
            }
        }

        saveables
    }

    /*
        A bufferable register is a saveable register that its lifetime
        does not cross an external call boundary, which can invalidate
        the buffer
    */
    fn find_bufferable(&self, linear: &Vec<Linear>) -> HashSet<Word> {
        let caller = [
            "rem", "power", "sin", "cos", "tan", "csc", "sec", "cot", "arcsin", "arccos", "arctan",
            "exp", "ln", "log", "root",
        ];

        let mut candidates: Vec<Word> = Vec::new();
        let mut bufferable: HashSet<Word> = HashSet::new();

        for l in linear.iter() {
            match l {
                Linear::Producer(p) => {
                    candidates.push(*p);
                }
                Linear::Consumer(c) => {
                    let r = candidates.pop();
                    if candidates.contains(c) {
                        bufferable.insert(*c);
                    };
                    if r.is_some() {
                        candidates.push(r.unwrap());
                    };
                }
                Linear::Caller(op) => {
                    if caller.contains(&op.as_str()) {
                        candidates.clear();
                    }
                }
            }
        }

        bufferable
    }

    fn load_buffered(&mut self, x: u8, r: Word) {
        if self.x4.is_some_and(|s| s == r) {
            self.push(format!("movsd xmm{}, xmm4", x).as_str());
            self.x4 = None;
            return;
        }

        if self.x5.is_some_and(|s| s == r) {
            self.push(format!("movsd xmm{}, xmm5", x).as_str());
            self.x5 = None;
            return;
        }

        self.load_xmm_indirect(x, r);
    }

    fn save_buffered(&mut self, x: u8, r: Word) {
        if self.x4.is_none() {
            self.push(format!("movsd xmm4, xmm{}", x).as_str());
            self.x4 = Some(r);
            return;
        }

        if self.x5.is_none() {
            self.push(format!("movsd xmm5, xmm{}", x).as_str());
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
}

impl Compiler<MachineCode> for NativeCompiler {
    fn compile(&mut self, prog: &Program) -> MachineCode {
        // function prelude
        self.push("push rbp");
        self.push("push rbx");

        self.push("mov rbp, rdi");
        self.push("mov rbx, rdx");

        let mut r = Word(0);

        let linear = self.linearize(prog);
        let saveables = self.find_saveables(&linear);

        let bufferable: HashSet<Word> = if self.optimize {
            self.find_bufferable(&linear)
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
                        self.push("movsd xmm1, xmm0");
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
                        self.push("movsd xmm2, xmm0");
                    } else {
                        self.load_buffered(Self::XMM2, *cond);
                    }

                    if *x2 == r {
                        self.push("movsd xmm1, xmm0");
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
                r = Word(0);
            }

            // A bufferable register can be buffered without the
            // need for self.dump_buffer()
            if self.optimize && bufferable.contains(&r) {
                self.save_buffered(Self::XMM0, r);
                r = Word(0);
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
                r = Word(0);
            }
        }

        // function closing instructions
        self.push("pop rbx");
        self.push("pop rbp");
        self.push("ret");

        MachineCode::new(
            &self.assembler.code(),
            prog.virtual_table(),
            prog.frame.mem(),
        )
    }
}

#[derive(Debug)]
pub struct MachineCode {
    p: *const u8,
    mmap: Mmap, // we need to store mmap and fs here, so that they are not dropped
    name: String,
    fs: fs::File,
    vt: Vec<BinaryFunc>,
    _mem: Vec<f64>,
}

impl MachineCode {
    fn new(machine_code: &Vec<u8>, vt: Vec<BinaryFunc>, _mem: Vec<f64>) -> MachineCode {
        let name = Alphanumeric.sample_string(&mut rand::thread_rng(), 16) + ".bin";
        MachineCode::write_buf(machine_code, &name);
        let fs = fs::File::open(&name).unwrap();
        let mmap = unsafe { MmapOptions::new().map_exec(&fs).unwrap() };
        let p = mmap.as_ptr() as *const u8;

        MachineCode {
            p,
            mmap,
            name,
            fs,
            vt,
            _mem,
        }
    }

    fn write_buf(machine_code: &Vec<u8>, name: &str) {
        let mut fs = fs::File::create(name).unwrap();
        fs.write(machine_code).unwrap();
    }
}

impl Compiled for MachineCode {
    fn run(&mut self) {
        let f: fn(&[f64], &[BinaryFunc]) = unsafe { std::mem::transmute(self.p) };
        f(&mut self._mem, &self.vt);
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

impl Drop for MachineCode {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.name);
    }
}
