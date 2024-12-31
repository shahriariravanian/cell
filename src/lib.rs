use std::ffi::{c_char, CStr, CString};

mod utils;
mod register;
mod model;
mod code;
mod amd;

use crate::utils::*;
use crate::model::{CellModel, Program};
use crate::amd::*;

pub struct Function {
    pub prog:           Program,
    pub compiled:       Box<dyn Compiled>,    
}

impl Function {
    pub fn new(mut prog: Program) -> Function {
        // prog.calc_virtual_table();
        // Function consumes Program
        let compiled = Box::new(NativeCompiler::new().compile(&prog));
        
        Function {
            prog,
            compiled,         
        }
    }

    pub fn run_mem(&self, mem: &mut [f64]) {        
        // self.compiled.run(mem, &self.prog.vt[..]);
        self.compiled.run(mem);
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CompilerStatus {
    Ok,
    Incomplete,
    InvalidUtf8,
    ParseError
}

pub struct CompilerResult { 
    func:   Option<Function>,
    regs:   CString,
    status: CompilerStatus,
}

#[no_mangle]
pub extern "C" fn compile(p: *const c_char) -> *const CompilerResult  {
    let mut res = CompilerResult { 
        func: None, 
        regs: CString::new("").unwrap(), 
        status: CompilerStatus::Incomplete 
    };

    let s = unsafe { 
        match CStr::from_ptr(p).to_str() {
            Ok(s) => s,
            Err(_) => {
                res.status = CompilerStatus::InvalidUtf8;
                return Box::into_raw(Box::new(res)) as *const _
            }
        }
    };

    let ml = match CellModel::load(&s) {
        Ok(ml) => ml,
        Err(_) => {
                res.status = CompilerStatus::ParseError;
                return Box::into_raw(Box::new(res)) as *const _
            }
    };
    
    let prog = Program::new(&ml);
    res.regs = CString::new(prog.frame.as_json().unwrap()).unwrap();
    res.func = Some(Function::new(prog));
    res.status = CompilerStatus::Ok;
    return Box::into_raw(Box::new(res)) as *const _
}


#[no_mangle]
pub extern "C" fn check_status(p: *const CompilerResult) -> *const c_char {
    let p: &CompilerResult = unsafe { &*p };
    let msg = match p.status {
        CompilerStatus::Ok =>           c"Success",
        CompilerStatus::Incomplete =>   c"Incomplete (internal error)",
        CompilerStatus::InvalidUtf8 =>  c"The input string is not valid UTF8",
        CompilerStatus::ParseError =>   c"Parse error",        
    };
    msg.as_ptr() as *const _
}

#[no_mangle]
pub extern "C" fn define_regs(p: *const CompilerResult) -> *const c_char {
    let p: &CompilerResult = unsafe { &*p };
    p.regs.as_ptr()
}


#[no_mangle]
pub extern "C" fn run(p: *mut CompilerResult, q: *mut f64, n: usize) {
    let p: &CompilerResult = unsafe { &*p };
    let mem: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(q, n) };
    /*
    for i in 0..n {
        mem[i] = i as f64;
    }
    */
    if let Some(func) = &p.func {
        func.run_mem(mem);
    }
    
}

#[no_mangle]
pub extern "C" fn finalize(p: *mut CompilerResult) {
    if !p.is_null() {    
        let _ = unsafe { Box::from_raw(p) };
    }
}

