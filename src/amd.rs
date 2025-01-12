use memmap2::{Mmap, MmapOptions};
use rand::distributions::{Alphanumeric, DistString};
use std::collections::HashSet;
use std::fs;
use std::io::Write;

use crate::code::*;
use crate::model::Program;
use crate::register::Reg;
use crate::utils::*;

#[derive(Debug)]
pub struct NativeCompiler {
    machine_code: Vec<u8>,
    optimize: bool,
    x4: Option<Reg>,
    x5: Option<Reg>,
}

pub enum Linear {
    Producer(Reg),
    Consumer(Reg),
    Caller(String),
}

#[derive(Debug)]
pub enum Comparison {
    Eq,
    NotEq,
    Greater,
    GreaterEq,
    Less,
    LessEq,
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
    const RAX: u8 = 0;
    const RCX: u8 = 1;
    const RDX: u8 = 2;
    const RBX: u8 = 3;
    const RSP: u8 = 4;
    const RBP: u8 = 5;
    const RSI: u8 = 6;
    const RDI: u8 = 7;
    const R8: u8 = 8;
    const R9: u8 = 9;
    const R10: u8 = 10;
    const R11: u8 = 11;
    const R12: u8 = 12;
    const R13: u8 = 13;
    const R14: u8 = 14;
    const R15: u8 = 15;

    pub fn new(optimize: bool) -> NativeCompiler {
        Self {
            machine_code: Vec::new(),
            x4: None,
            x5: None,
            optimize
        }
    }

    fn byte(&mut self, b: u8) {
        self.machine_code.push(b);
    }

    fn bytes(&mut self, bs: &[u8]) {
        self.machine_code.extend_from_slice(bs);
    }

    fn op_code(&mut self, op: &str, p: Proc) {
        match op {
            "mov" => {}
            "plus" => self.add(Self::XMM0, Self::XMM1),
            "minus" => self.sub(Self::XMM0, Self::XMM1),
            "times" => self.mul(Self::XMM0, Self::XMM1),
            "divide" => self.div(Self::XMM0, Self::XMM1),
            "gt" => self.comparison(Comparison::Greater, Self::XMM0, Self::XMM1),
            "geq" => self.comparison(Comparison::GreaterEq, Self::XMM0, Self::XMM1),
            "lt" => self.comparison(Comparison::Less, Self::XMM0, Self::XMM1),
            "leq" => self.comparison(Comparison::LessEq, Self::XMM0, Self::XMM1),
            "eq" => self.comparison(Comparison::Eq, Self::XMM0, Self::XMM1),
            "neq" => self.comparison(Comparison::NotEq, Self::XMM0, Self::XMM1),
            "and" => self.and(Self::XMM0, Self::XMM1),
            "or" => self.or(Self::XMM0, Self::XMM1),
            "xor" => self.xor(Self::XMM0, Self::XMM1),
            "neg" => {
                self.load_xmm_indirect(Self::XMM1, Reg(3));
                self.xor(Self::XMM0, Self::XMM1);
            }
            _ => {                
                if !self.optimize {
                    self.dump_buffer();
                }
                self.load_reg_indirect(Self::RAX, Self::RBX, 8 * p.0); // mov    rax,QWORD PTR [rbx+8*f]
                self.call_reg(Self::RAX); // call   rax
            }
        }
    }

    fn comparison(&mut self, cmp: Comparison, dst: u8, src: u8) {
        self.bytes(&[0xf2, 0x0f, 0xc2]);
        self.modrm_reg(src, dst);

        let code = match cmp {
            Comparison::Eq => 0,
            Comparison::NotEq => 4,
            Comparison::Greater => 6, // strictly, this is not-less-than-or-equal
            Comparison::GreaterEq => 5, // strictly, this is not-less-than
            Comparison::Less => 1,
            Comparison::LessEq => 2,
        };

        self.byte(code);
    }

    // xmm2 == true ? xmm0 : xmm1
    fn ifelse(&mut self) {
        self.move_xmm(Self::XMM3, Self::XMM2);
        self.and(Self::XMM0, Self::XMM2);
        self.andnot(Self::XMM3, Self::XMM1);
        self.add(Self::XMM0, Self::XMM3);
    }

    fn add(&mut self, dst: u8, src: u8) {
        self.bytes(&[0xf2, 0x0f, 0x58]);
        self.modrm_reg(src, dst);
    }

    fn sub(&mut self, dst: u8, src: u8) {
        self.bytes(&[0xf2, 0x0f, 0x5c]);
        self.modrm_reg(src, dst);
    }

    fn mul(&mut self, dst: u8, src: u8) {
        self.bytes(&[0xf2, 0x0f, 0x59]);
        self.modrm_reg(src, dst);
    }

    fn div(&mut self, dst: u8, src: u8) {
        self.bytes(&[0xf2, 0x0f, 0x5e]);
        self.modrm_reg(src, dst);
    }

    fn and(&mut self, dst: u8, src: u8) {
        self.bytes(&[0x66, 0x0f, 0x54]);
        self.modrm_reg(src, dst);
    }

    fn andnot(&mut self, dst: u8, src: u8) {
        self.bytes(&[0x66, 0x0f, 0x55]);
        self.modrm_reg(src, dst);
    }

    fn or(&mut self, dst: u8, src: u8) {
        self.bytes(&[0x66, 0x0f, 0x56]);
        self.modrm_reg(src, dst);
    }

    fn xor(&mut self, dst: u8, src: u8) {
        self.bytes(&[0x66, 0x0f, 0x57]);
        self.modrm_reg(src, dst);
    }

    fn sqrt(&mut self, dst: u8, src: u8) {
        self.bytes(&[0xf2, 0x0f, 0x51]);
        self.modrm_reg(src, dst);
    }

    fn push_reg(&mut self, r: u8) {
        if r < 8 {
            self.byte(0x50 + r);
        } else {
            self.byte(0x41);
            self.byte(0x48 + r);
        }
    }

    fn pop_reg(&mut self, r: u8) {
        if r < 8 {
            self.byte(0x58 + r);
        } else {
            self.byte(0x41);
            self.byte(0x50 + r);
        }
    }

    fn ret(&mut self) {
        self.byte(0xc3);
    }

    fn call_reg(&mut self, r: u8) {
        self.byte(0xff);
        self.byte(0xd0 + r);
    }

    fn modrm_reg(&mut self, dst: u8, src: u8) {
        self.byte(0xC0 + (src << 3) + dst);
    }

    fn move_reg(&mut self, dst: u8, src: u8) {
        self.byte(0x48); // REX
        self.byte(0x89); // MOV
        self.modrm_reg(dst, src);
    }

    fn modrm_mem(&mut self, dst: u8, base: u8, offset: usize) {
        if offset < 128 {
            // note: disp8 is 2's complement
            self.byte(0x40 + (dst << 3) + base);
            self.byte(offset as u8);
        } else {
            self.byte(0x80 + (dst << 3) + base);
            self.byte(offset as u8);
            self.byte((offset >> 8) as u8);
            self.byte((offset >> 16) as u8);
            self.byte((offset >> 24) as u8);
        }
    }

    fn load_reg_indirect(&mut self, r: u8, base: u8, offset: usize) {
        self.byte(0x48); // REX
        self.byte(0x8b); // MOV
        self.modrm_mem(r, base, offset);
    }

    fn save_reg_indirect(&mut self, r: u8, base: u8, offset: usize) {
        self.byte(0x48); // REX
        self.byte(0x89); // MOV
        self.modrm_mem(r, base, offset);
    }

    fn move_xmm(&mut self, dst: u8, src: u8) {
        self.bytes(&[0xf2, 0x0f, 0x10]);
        self.modrm_reg(src, dst);
    }

    // movq XMM[dst], GP[src]
    fn move_xmm_reg(&mut self, dst: u8, src: u8) {
        self.bytes(&[0x66, 0x48 | (src >> 3), 0x0f, 0x6e]);
        self.modrm_reg(src & 7, dst);
    }

    // movq GP[dst], XMM[src]
    fn move_reg_xmm(&mut self, dst: u8, src: u8) {
        self.bytes(&[0x66, 0x48 | (dst >> 3), 0x0f, 0x7e]);
        self.modrm_reg(dst & 7, src);
    }

    fn load_xmm_indirect(&mut self, x: u8, r: Reg) {
        if r == Reg(0) {
            self.xor(x, x);
        } else {
            self.load_xmm(x, Self::RBP, 8 * r.0);
        }
    }

    fn save_xmm_indirect(&mut self, x: u8, r: Reg) {
        if r.0 > 2 {
            self.save_xmm(x, Self::RBP, 8 * r.0);
        }
    }

    fn load_xmm(&mut self, x: u8, base: u8, offset: usize) {
        self.bytes(&[0xf2, 0x0f, 0x10]);
        self.modrm_mem(x, base, offset);
    }

    fn save_xmm(&mut self, x: u8, base: u8, offset: usize) {
        self.bytes(&[0xf2, 0x0f, 0x11]);
        self.modrm_mem(x, base, offset);
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
        };
        linear
    }

    /*
        A saveable register is produced but is not consumed immediately
        In other words, it cannot be coalesced over consecuative instructions
    */
    fn find_saveables(&self, linear: &Vec<Linear>) -> HashSet<Reg> {
        let mut candidates: Vec<Reg> = Vec::new();
        let mut saveables: HashSet<Reg> = HashSet::new();        

        for l in linear.iter() {
            match l {
                Linear::Producer(p) => { 
                    candidates.push(*p);
                },
                Linear::Consumer(c) => {       
                    let r = candidates.pop();
                    if candidates.contains(c) {
                        saveables.insert(*c);
                    }; 
                    if r.is_some() {
                        candidates.push(r.unwrap());
                    };                    
                },
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
    fn find_bufferable(&self, linear: &Vec<Linear>) -> HashSet<Reg> {
        let caller = ["rem", "power", 
                      "sin", "cos", "tan", 
                      "csc", "sec", "cot", 
                      "arcsin", "arccos", "arctan", 
                      "exp", "ln", "log", "root"];
        
        let mut candidates: Vec<Reg> = Vec::new();
        let mut bufferable: HashSet<Reg> = HashSet::new();        

        for l in linear.iter() {
            match l {
                Linear::Producer(p) => { 
                    candidates.push(*p);
                },
                Linear::Consumer(c) => {       
                    let r = candidates.pop();
                    if candidates.contains(c) {
                        bufferable.insert(*c);
                    }; 
                    if r.is_some() {
                        candidates.push(r.unwrap());
                    };                    
                },
                Linear::Caller(op) => {
                    if caller.contains(&op.as_str()) {
                        candidates.clear();
                    }
                }
            }        
        }

        bufferable
    }

    fn load_buffered(&mut self, x: u8, r: Reg) {
        if self.x4.is_some_and(|s| s == r) {
            self.move_xmm(x, Self::XMM4);
            self.x4 = None;
            return;
        }

        if self.x5.is_some_and(|s| s == r) {
            self.move_xmm(x, Self::XMM5);
            self.x5 = None;
            return;
        }

        self.load_xmm_indirect(x, r);
    }

    fn save_buffered(&mut self, x: u8, r: Reg) {
        if self.x4.is_none() {
            self.move_xmm(Self::XMM4, x);
            self.x4 = Some(r);
            return;
        }

        if self.x5.is_none() {
            self.move_xmm(Self::XMM5, x);
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
        self.push_reg(Self::RBP); // push   rbp
        self.push_reg(Self::RBX); // push   rbx

        self.move_reg(Self::RBP, Self::RDI); // mov    rbp,rdi
        self.move_reg(Self::RBX, Self::RDX); // mov    rbx,rdx

        let mut r = Reg(0);
        
        let linear = self.linearize(prog);
        let saveables = self.find_saveables(&linear);
        
        let bufferable: HashSet<Reg> = if self.optimize {
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
                        self.move_xmm(Self::XMM1, Self::XMM0);
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
                        self.move_xmm(Self::XMM2, Self::XMM0);
                    } else {
                        self.load_buffered(Self::XMM2, *cond);
                    }

                    if *x2 == r {
                        self.move_xmm(Self::XMM1, Self::XMM0);
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
                r = Reg(0);
            }
            
            
            // A bufferable register can be buffered without the
            // need for self.dump_buffer()            
            if self.optimize && bufferable.contains(&r) {
                self.save_buffered(Self::XMM0, r);
                r = Reg(0);
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
                r = Reg(0);
            }
        }

        // function closing instructions
        self.pop_reg(Self::RBX); // pop    rbx
        self.pop_reg(Self::RBP); // pop    rbp
        self.ret(); // ret

        MachineCode::new(&self.machine_code, prog.virtual_table(), prog.frame.mem())
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
        // let _ = fs::remove_file(&self.name);
    }
}
