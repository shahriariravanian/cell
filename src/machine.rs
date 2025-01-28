use memmap2::{Mmap, MmapOptions};
use rand::distributions::{Alphanumeric, DistString};
use std::fs;
use std::io::Write;

use super::code::BinaryFunc;
use super::utils::*;

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
    pub fn new(machine_code: &Vec<u8>, vt: Vec<BinaryFunc>, _mem: Vec<f64>) -> MachineCode {
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
