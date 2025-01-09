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
    buf: Vec<u8>,
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

    pub fn new() -> NativeCompiler {
        Self { buf: Vec::new() }
    }

    fn byte(&mut self, b: u8) {
        self.buf.push(b);
    }

    fn bytes(&mut self, bs: &[u8]) {
        self.buf.extend_from_slice(bs);
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
                self.load_xmm_reg(Self::XMM1, Reg(3));
                self.xor(Self::XMM0, Self::XMM1);
            }
            _ => {
                // println!("{:x}:\t{}", self.buf.len(), op);
                self.load_reg(Self::RAX, Self::RBX, 8 * p.0); // mov    rax,QWORD PTR [rbx+8*f]
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

    fn push_reg(&mut self, r: u8) {
        self.byte(0x50 + r);
    }

    fn pop_reg(&mut self, r: u8) {
        self.byte(0x58 + r);
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

    fn load_reg(&mut self, r: u8, base: u8, offset: usize) {
        self.byte(0x48); // REX
        self.byte(0x8b); // MOV
        self.modrm_mem(r, base, offset);
    }

    fn move_xmm(&mut self, dst: u8, src: u8) {
        self.bytes(&[0xf2, 0x0f, 0x10]);
        self.modrm_reg(src, dst);
    }

    fn load_xmm_reg(&mut self, r: u8, x: Reg) {
        if x == Reg(0) {
            self.xor(r, r);
        } else {
            self.load_xmm(r, Self::RBP, 8 * x.0);
        }
    }

    fn save_xmm_reg(&mut self, r: u8, x: Reg) {
        if x.0 > 2 {
            self.save_xmm(r, Self::RBP, 8 * x.0);
        }
    }

    fn load_xmm(&mut self, r: u8, base: u8, offset: usize) {
        self.byte(0xf2);
        self.byte(0x0f);
        self.byte(0x10);
        self.modrm_mem(r, base, offset);
    }

    fn save_xmm(&mut self, r: u8, base: u8, offset: usize) {
        self.byte(0xf2);
        self.byte(0x0f);
        self.byte(0x11);
        self.modrm_mem(r, base, offset);
    }

    fn find_saveables(&self, prog: &Program) -> HashSet<Reg> {
        let mut saveables: HashSet<Reg> = HashSet::new();

        let mut r = Reg(0);

        for c in prog.code.iter() {
            match c {
                Instruction::Unary { x, dst, .. } => {
                    if *x != r {
                        saveables.insert(*x);
                    }
                    r = *dst;
                }
                Instruction::Binary { x, y, dst, .. } => {
                    if *x != r {
                        saveables.insert(*x);
                    }
                    if *y != r {
                        saveables.insert(*y);
                    }
                    r = *dst;
                }
                Instruction::IfElse { x1, x2, cond, dst } => {
                    if *x1 != r {
                        saveables.insert(*x1);
                    }
                    if *x2 != r {
                        saveables.insert(*x2);
                    }
                    if *cond != r {
                        saveables.insert(*cond);
                    }
                    r = *dst;
                }
                _ => {}
            }
        }

        saveables
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
        let saveables = self.find_saveables(prog);

        for c in prog.code.iter() {
            match c {
                Instruction::Unary { p, x, dst, op } => {
                    if r != *x {
                        self.load_xmm_reg(Self::XMM0, *x);
                    };
                    self.op_code(&op, *p);
                    r = *dst;
                }
                Instruction::Binary { p, x, y, dst, op } => {
                    if *y == r {
                        self.move_xmm(Self::XMM1, Self::XMM0);
                    } else {
                        self.load_xmm_reg(Self::XMM1, *y);
                    }

                    if *x != r {
                        self.load_xmm_reg(Self::XMM0, *x);
                    }

                    self.op_code(&op, *p);
                    r = *dst;
                }
                Instruction::IfElse { x1, x2, cond, dst } => {
                    if *cond == r {
                        self.move_xmm(Self::XMM2, Self::XMM0);
                    } else {
                        self.load_xmm_reg(Self::XMM2, *cond);
                    }
                
                    if *x2 == r {
                        self.move_xmm(Self::XMM1, Self::XMM0);
                    } else {
                        self.load_xmm_reg(Self::XMM1, *x2);
                    }

                    if *x1 != r {
                        self.load_xmm_reg(Self::XMM0, *x1);
                    }

                    self.ifelse();
                    r = *dst;
                }
                _ => {}
            }

            // only save the result if it is part of the function output (is_diff)
            // or is needed by instructions after the immediate next one
            if prog.frame.is_diff(&r) || saveables.contains(&r) {
                self.save_xmm_reg(Self::XMM0, r);
            }
        }

        // function closing instructions
        self.pop_reg(Self::RBX); // pop    rbx
        self.pop_reg(Self::RBP); // pop    rbp
        self.ret(); // ret

        MachineCode::new(&self.buf, prog.virtual_table(), prog.frame.mem())
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
    fn new(buf: &Vec<u8>, vt: Vec<BinaryFunc>, _mem: Vec<f64>) -> MachineCode {
        let name = Alphanumeric.sample_string(&mut rand::thread_rng(), 16) + ".bin";
        MachineCode::write_buf(buf, &name);
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

    fn write_buf(buf: &Vec<u8>, name: &str) {
        let mut fs = fs::File::create(name).unwrap();
        fs.write(buf).unwrap();
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
