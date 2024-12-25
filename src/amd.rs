use std::fs::File;
use std::io::Write;
use memmap2::{MmapOptions, Mmap};

use crate::code::*;
use crate::model::Program;
use crate::register::Reg;

#[derive(Debug)]
pub struct Amd64 {
    pub buf: Vec<u8>
}

impl Amd64 {
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
    
    pub fn new() -> Amd64 {
        Self {
            buf: Vec::new()
        }
    }
    
    pub fn compile(&mut self, prog: &Program) -> Compiled {
        self.push_reg(Self::RBP);                           // push   rbp
        self.push_reg(Self::RBX);                           // push   rbx
        
        self.move_reg(Self::RBP, Self::RDI);                // mov    rbp,rdi
        self.move_reg(Self::RBX, Self::RDX);                // mov    rbx,rdx        
        
        let mut r = Reg(0);
        
        for c in prog.code.iter()  {
            match c {
                Instruction::Num {..} => {},                        // Num and Var do not generate any code 
                Instruction::Var {..} => {},                        // They are mainly for debugging
                Instruction::Op {p, x, y, dst, op} => { 
                    /*
                        self.load_xmm(Self::XMM0, Self::RBP, 8*x.0);    // movsd  xmm0,QWORD PTR [rbp+8*x]
                        self.load_xmm(Self::XMM1, Self::RBP, 8*y.0);    // movsd  xmm1,QWORD PTR [rbp+8*y]
                        self.op_code(&op, *p);
                        self.save_xmm(Self::XMM0, Self::RBP, 8*dst.0);  // movsd  QWORD PTR [rbp+8*dst],xmm0
                    */
                    if op == "mov" {
                        if r != *x {
                            self.save_xmm_reg(Self::XMM0, r);                            
                            self.load_xmm_reg(Self::XMM0, *x);
                        }
                    } else {
                        if r == *x {
                            self.load_xmm_reg(Self::XMM1, *y);    
                        } else if r == *y {
                            self.move_xmm(Self::XMM1, Self::XMM0);
                            self.load_xmm_reg(Self::XMM0, *x);
                        } else {
                            self.save_xmm_reg(Self::XMM0, r);
                            self.load_xmm_reg(Self::XMM0, *x);
                            self.load_xmm_reg(Self::XMM1, *y);
                        }
                        self.op_code(&op, *p);
                    }
                    r = *dst;
                }
            }
        }               
        
        self.save_xmm_reg(Self::XMM0, r); 
        
        self.pop_reg(Self::RBX);                            // pop    rbx
        self.pop_reg(Self::RBP);                            // pop    rbp
        self.ret();                                         // ret
        
        Compiled::new(&self.buf)
    }
    
    fn op_code(&mut self, op: &str, p: Proc) {
        match op {
            "mov" =>    {},
            "plus" =>   self.buf.extend_from_slice(&[0xf2, 0x0f, 0x58, 0xc1]),
            "minus" =>  self.buf.extend_from_slice(&[0xf2, 0x0f, 0x5c, 0xc1]),
            "times" =>  self.buf.extend_from_slice(&[0xf2, 0x0f, 0x59, 0xc1]),
            "divide" => self.buf.extend_from_slice(&[0xf2, 0x0f, 0x5e, 0xc1]),            
            "gt" =>     self.comparison(0x77),      // ja
            "geq" =>    self.comparison(0x73),      // jae
            "lt" =>     self.comparison(0x72),      // jb
            "leq" =>    self.comparison(0x76),      // jbe
            "eq" =>     self.comparison(0x74),      // je
            "neq" =>    self.comparison(0x75),      // jne
            _ => {
                // println!("{:x}:\t{}", self.buf.len(), op);
                self.load_reg(Self::RAX, Self::RBX, 8*p.0);     // mov    rax,QWORD PTR [rbx+8*f]
                self.call_reg(Self::RAX);                       // call   rax                            
            }
        }
    }    
    
    fn comparison(&mut self, code: u8) {
        self.buf.extend_from_slice(&[0x66, 0x0f, 0x2e, 0xc1]);  // ucomisd xmm0, xmm1
        self.buf.push(code);                        // Jx code
        self.buf.push(0x07);                        // jump 5 + 2 bytes
        self.load_xmm(Self::XMM0, Self::RBP, 16);   // Reg(2) = -1
        self.buf.push(0xeb);                        // JMP
        self.buf.push(0x05);                        // jump 5 bytes
        self.load_xmm(Self::XMM0, Self::RBP, 8);    // Reg(1) = 1
    }
    
    fn push_reg(&mut self, r: u8) {
        self.buf.push(0x50 + r);
    }
    
    fn pop_reg(&mut self, r: u8) {
        self.buf.push(0x58 + r);
    }
    
    fn ret(&mut self) {
        self.buf.push(0xc3);
    }
    
    fn call_reg(&mut self, r: u8) {
        self.buf.push(0xff);
        self.buf.push(0xd0 + r);
    }
    
    fn modrm_reg(&mut self, dst: u8, src: u8) {
        self.buf.push(0xC0 + (src << 3) + dst);
    }
    
    fn move_reg(&mut self, dst: u8, src: u8) {
        self.buf.push(0x48);    // REX
        self.buf.push(0x89);    // MOV
        self.modrm_reg(dst, src);
    }   
    
    fn modrm_mem(&mut self, dst: u8, base: u8, offset: usize) {
        if offset < 128 {   // note: disp8 is 2's complement
            self.buf.push(0x40 + (dst << 3) + base);
            self.buf.push(offset as u8);
        } else {
            self.buf.push(0x80 + (dst << 3) + base);
            self.buf.push(offset as u8);
            self.buf.push((offset >> 8) as u8);
            self.buf.push((offset >> 16) as u8);
            self.buf.push((offset >> 24) as u8);
        }        
    }    
    
    fn load_reg(&mut self, r: u8, base: u8, offset: usize) {
        self.buf.push(0x48);    // REX
        self.buf.push(0x8b);    // MOV
        self.modrm_mem(r, base, offset);
    }    
    
    fn move_xmm(&mut self, dst: u8, src: u8) {
        self.buf.extend_from_slice(&[0xf2, 0x0f, 0x10]);
        self.modrm_reg(src, dst);
    }
    
    fn load_xmm_reg(&mut self, r: u8, x: Reg) {
        if x == Reg(0) {
            self.buf.push(0x66);    // XORPD r, r
            self.buf.push(0x0f);
            self.buf.push(0x57);
            self.modrm_reg(r, r);
        } else {
            self.load_xmm(r, Self::RBP, 8*x.0);
        }
    }
    
    fn save_xmm_reg(&mut self, r: u8, x: Reg) {
        if x.0 > 2 {
            self.save_xmm(r, Self::RBP, 8*x.0);
        }
    }
    
    fn load_xmm(&mut self, r: u8, base: u8, offset: usize) {
        self.buf.push(0xf2);
        self.buf.push(0x0f);
        self.buf.push(0x10);
        self.modrm_mem(r, base, offset);
    }
    
    fn save_xmm(&mut self, r: u8, base: u8, offset: usize) {
        self.buf.push(0xf2);
        self.buf.push(0x0f);
        self.buf.push(0x11);
        self.modrm_mem(r, base, offset);
    }
}


#[derive(Debug)]
pub struct Compiled {
    p: *const u8,
    mmap: Mmap,     // we need to store mmap and fs here, so that they are not dropped
    fs: File,
}

impl Compiled {
    fn new(buf: &Vec<u8>) -> Compiled {
        Compiled::write_buf(buf);
        let fs = File::open("code.bin").unwrap();
        let mmap = unsafe { MmapOptions::new().map_exec(&fs).unwrap() };
        let p = mmap.as_ptr() as *const u8;
        
        Compiled{
            p,
            mmap,
            fs
        }
    }
    
    fn write_buf(buf: &Vec<u8>) {
        let mut fs = File::create("code.bin").unwrap();
        fs.write(buf).unwrap();     
    }
    
    pub fn run(&self, mem: &mut Vec<f64>, vt: &Vec<fn (f64, f64) -> f64>) {    
        let f: fn (&[f64], &[fn(f64, f64) -> f64])  = unsafe { std::mem::transmute(self.p) };    
        f(&mut mem[..], &vt[..]);                
    }
}


