use std::fs::File;
use std::io::Write;
use memmap2::{MmapOptions, Mmap};

use crate::code::*;

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
    
    pub fn compile(&mut self, inter: &Intermediate) -> Compiled {
        self.push_reg(Self::RBP);
        self.push_reg(Self::RBX);
        
        self.move_reg(Self::RBP, Self::RDI);
        self.move_reg(Self::RBX, Self::RDX);
        
        for (f, dst, x, y) in inter.tac.iter() {
            self.load_xmm(Self::XMM0, Self::RBP, 8*x);
            self.load_xmm(Self::XMM1, Self::RBP, 8*y);
            self.load_reg(Self::RAX, Self::RBX, 8*f);
            self.call_reg(Self::RAX);
            self.save_xmm(Self::XMM0, Self::RBP, 8*dst);
        }                
        
        self.pop_reg(Self::RBX);
        self.pop_reg(Self::RBP);
        self.ret();
        
        Compiled::new(&self.buf)
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
    
    pub fn run(&self, mem: &Vec<f64>, vt: &Vec<fn (f64, f64) -> f64>) {    
        let f: fn (&[f64], &[fn(f64, f64) -> f64])  = unsafe { std::mem::transmute(self.p) };    
        f(&mem[..], &vt[..]);                
    }
}


