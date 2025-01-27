use memmap2::{Mmap, MmapOptions};
use rand::distributions::{Alphanumeric, DistString};
use std::collections::HashSet;
use std::fs;
use std::io::Write;

#[macro_use] mod macros;
mod assembler;

use super::code::*;
use super::model::Program;
use super::register::{Frame, Word};
use super::utils::*;
use assembler::Assembler;

#[derive(Debug)]
pub struct ArmCompiler {
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

impl ArmCompiler {
    const D0: u8 = 0;
    const D1: u8 = 1;
    const D2: u8 = 2;
    const D3: u8 = 3;
    const D4: u8 = 4;
    const D5: u8 = 5;
    const D6: u8 = 6;
    const D7: u8 = 7;

    pub fn new(optimize: bool) -> ArmCompiler {
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
    
    fn push_u32(&mut self, w: u32) {
        self.assembler.push_u32(w);
    } 

    fn op_code(&mut self, op: &str, p: Proc) {
        match op {
            "mov" => {}
            //"plus" => self.push("fadd d0, d0, d1"),
            //"minus" => self.push("fsub d0, d0, d1"),
            //"times" => self.push("fmul d0, d0, d1"),
            //"divide" => self.push("fdiv d0, d0, d1"),
            "plus" => self.push_u32(arm!{fadd d(0), d(0), d(1)}),
            "minus" => self.push_u32(arm!{fsub d(0), d(0), d(1)}),
            "times" => self.push_u32(arm!{fmul d(0), d(0), d(1)}),
            "divide" => self.push_u32(arm!{fdiv d(0), d(0), d(1)}),
            "gt" => self.push("fcmgt d0, d0, d1"),
            "geq" => self.push("fcmge d0, d0, d1"),
            "lt" => self.push("fcmlt d0, d0, d1"),
            "leq" => self.push("fcmle d0, d0, d1"),
            "eq" => self.push("cmpeq d0, d0, d1"),
            "and" => self.push("and v0.8b, v0.8b, v0.8b"),
            "or" => self.push("orr v0.8b, v0.8b, v0.8b"),
            "xor" => self.push("eor v0.8b, v0.8b, v0.8b"),
            "neg" => self.push("fneg d0, d0"),
            "root" => self.push("fsqrt d0, d0"),
            "neq" => {
                self.push("cmpeq d0, d0, d1");
                self.push("not d0, d0");
            }
            _ => {
                if !self.optimize {
                    self.dump_buffer();
                }
                self.push(format!("ldr x0, [x20, #{}]; op = {}", 8 * p.0, op).as_str());
                self.push("blr x0");
            }
        }
    }

    // d2 == true ? d0 : d1
    fn ifelse(&mut self) {
        self.push("and v0.8b, v0.8b, v2.8b");
        self.push("not v2.8b, v2.8b");
        self.push("and v1.8b, v1.8b, v2.8b");
        self.push("orr v0.8b, v0.8b, v1.8b");
    }

    fn load_xmm_indirect(&mut self, x: u8, r: Word) {
        if r == Frame::ZERO {
            self.push(format!("fmov d{}, #0.0", x).as_str());
        } else if r == Frame::ONE {
            self.push(format!("fmov d{}, #1.0", x).as_str());
        } else if r == Frame::MINUS_ONE {
            self.push(format!("fmov d{}, #-1.0", x).as_str());
        } else {
            self.push(format!("ldr d{}, [x19, #{}]", x, 8 * r.0).as_str());
        }
    }

    fn save_xmm_indirect(&mut self, x: u8, r: Word) {
        if r.0 > 2 {
            self.push(format!("str d{}, [x19, #{}]", x, 8 * r.0).as_str());
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
            self.push(format!("fmov d{}, d4", x).as_str());
            self.x4 = None;
            return;
        }

        if self.x5.is_some_and(|s| s == r) {
            self.push(format!("fmov d{}, d5", x).as_str());
            self.x5 = None;
            return;
        }

        self.load_xmm_indirect(x, r);
    }

    fn save_buffered(&mut self, x: u8, r: Word) {
        if self.x4.is_none() {
            self.push(format!("fmov d4, d{}", x).as_str());
            self.x4 = Some(r);
            return;
        }

        if self.x5.is_none() {
            self.push(format!("fmov d5, d{}", x).as_str());
            self.x5 = Some(r);
            return;
        }

        self.save_xmm_indirect(x, r);
    }

    fn dump_buffer(&mut self) {
        if let Some(s) = self.x4 {
            self.save_xmm_indirect(Self::D4, s);
            self.x4 = None;
        }

        if let Some(s) = self.x5 {
            self.save_xmm_indirect(Self::D5, s);
            self.x5 = None;
        }
    }
}

impl Compiler<MachineCode> for ArmCompiler {
    fn compile(&mut self, prog: &Program) -> MachineCode {
        // function prelude
        self.push("sub sp, sp, #32");
        self.push("str lr, [sp, #0]");
        self.push("stp x19, x20, [sp, #16]");

        self.push("mov x19, x0");
        self.push("mov x20, x2");

        let mut r = Frame::ZERO;

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
                        self.load_buffered(Self::D0, *x);
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
                        self.push("fmov d1, d0 ; binary::y");
                    } else {
                        self.load_buffered(Self::D1, *y);
                    }

                    if *x != r {
                        self.load_buffered(Self::D0, *x);
                    }

                    self.op_code(&op, *p);
                    r = *dst;
                }
                Instruction::IfElse { x1, x2, cond, dst } => {
                    if *cond == r {
                        self.push("fmov d2, d0; ifelse::cond");
                    } else {
                        self.load_buffered(Self::D2, *cond);
                    }

                    if *x2 == r {
                        self.push("fmov d1, d0; ifelse::x2");
                    } else {
                        self.load_buffered(Self::D1, *x2);
                    }

                    if *x1 != r {
                        self.load_buffered(Self::D0, *x1);
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
            if self.optimize && bufferable.contains(&r) {
                self.save_buffered(Self::D0, r);
                r = Frame::ZERO;
            }

            // A saveable register can be saved directly or buffered
            // However, if it is buffered, self.dump_buffer() should be
            // uncommented in fn op_code
            if saveables.contains(&r) {
                if self.optimize {
                    self.save_xmm_indirect(Self::D0, r);
                } else {
                    self.save_buffered(Self::D0, r);
                }
                r = Frame::ZERO;
            }
        }

        // function closing instructions
        self.push("ldp x19, x20, [sp, #16]");
        self.push("ldr lr, [sp, #0]");
        self.push("add sp, sp, #32");
        self.push("ret");

        println!("{}", &self.assembler);

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
