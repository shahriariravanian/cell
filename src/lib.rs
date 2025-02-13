use std::ffi::{c_char, CStr, CString};

mod analyzer;
mod code;
mod machine;
mod model;
mod register;
mod runnable;
mod utils;

mod amd;
mod arm;
mod interpreter;
#[cfg(feature = "rusty")]
mod rusty;
#[cfg(feature = "wasm")]
mod wasm;

use model::{CellModel, Program};
use runnable::{CompilerType, Runnable};
use utils::*;

#[derive(Debug, Clone, Copy)]
pub enum CompilerStatus {
    Ok,
    Incomplete,
    InvalidUtf8,
    ParseError,
    InvalidCompiler,
}

pub struct CompilerResult {
    func: Option<Runnable>,
    regs: CString,
    status: CompilerStatus,
}

#[no_mangle]
pub extern "C" fn compile(p: *const c_char, ty: *const c_char) -> *const CompilerResult {
    let mut res = CompilerResult {
        func: None,
        regs: CString::new("").unwrap(),
        status: CompilerStatus::Incomplete,
    };

    let p = unsafe {
        match CStr::from_ptr(p).to_str() {
            Ok(p) => p,
            Err(_) => {
                res.status = CompilerStatus::InvalidUtf8;
                return Box::into_raw(Box::new(res)) as *const _;
            }
        }
    };

    let ty = unsafe {
        match CStr::from_ptr(ty).to_str() {
            Ok(ty) => ty,
            Err(_) => {
                res.status = CompilerStatus::InvalidUtf8;
                return Box::into_raw(Box::new(res)) as *const _;
            }
        }
    };

    let ml = match CellModel::load(&p) {
        Ok(ml) => ml,
        Err(_) => {
            res.status = CompilerStatus::ParseError;
            return Box::into_raw(Box::new(res)) as *const _;
        }
    };

    let prog = Program::new(&ml);

    // println!("{:#?}", &prog);

    res.func = match ty {
        "bytecode" => Some(Runnable::new(prog, CompilerType::ByteCode)),
        "arm" => Some(Runnable::new(prog, CompilerType::Arm)),
        "amd" => Some(Runnable::new(prog, CompilerType::Amd)),
        "native" => Some(Runnable::new(prog, CompilerType::Native)),
        #[cfg(feature = "wasm")]
        "wasm" => Some(Runnable::new(prog, CompilerType::Wasm)),
        #[cfg(feature = "rusty")]
        "rusty" => Some(Runnable::new(prog, CompilerType::Rusty)),
        _ => None,
    };

    res.status = if res.func.is_none() {
        CompilerStatus::InvalidCompiler
    } else {
        CompilerStatus::Ok
    };
    return Box::into_raw(Box::new(res)) as *const _;
}

#[no_mangle]
pub extern "C" fn check_status(q: *const CompilerResult) -> *const c_char {
    let q: &CompilerResult = unsafe { &*q };
    let msg = match q.status {
        CompilerStatus::Ok => c"Success",
        CompilerStatus::Incomplete => c"Incomplete (internal error)",
        CompilerStatus::InvalidUtf8 => c"The input string is not valid UTF8",
        CompilerStatus::ParseError => c"Parse error",
        CompilerStatus::InvalidCompiler => c"Compiler type not found",
    };
    msg.as_ptr() as *const _
}

#[no_mangle]
pub extern "C" fn count_states(q: *const CompilerResult) -> usize {
    let q: &CompilerResult = unsafe { &*q };
    if let Some(func) = &q.func {
        func.count_states
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn count_params(q: *const CompilerResult) -> usize {
    let q: &CompilerResult = unsafe { &*q };
    if let Some(func) = &q.func {
        func.count_params
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn fill_u0(q: *const CompilerResult, u0: *mut f64, ns: usize) -> bool {
    let q: &CompilerResult = unsafe { &*q };
    if let Some(func) = &q.func {
        if func.count_states != ns {
            return false;
        }

        let u0: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(u0, ns) };
        u0.copy_from_slice(&func.u0);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn fill_p(q: *const CompilerResult, p: *mut f64, np: usize) -> bool {
    let q: &CompilerResult = unsafe { &*q };
    if let Some(func) = &q.func {
        if func.count_params != np {
            return false;
        }

        let p: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(p, np) };
        p.copy_from_slice(&func.p);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn run(
    q: *mut CompilerResult,
    du: *mut f64,
    u: *const f64,
    ns: usize,
    p: *const f64,
    np: usize,
    t: f64,
) -> bool {
    let q: &mut CompilerResult = unsafe { &mut *q };

    if let Some(func) = &mut q.func {
        if func.count_states != ns || func.count_params != np {
            return false;
        }

        let du: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(du, ns) };
        let u: &[f64] = unsafe { std::slice::from_raw_parts(u, ns) };
        let p: &[f64] = unsafe { std::slice::from_raw_parts(p, np) };
        func.call(du, u, p, t);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn run_py(
    q: *mut CompilerResult,
    du: *mut f64,
    nd: usize,
    u: *const f64,
    ns: usize,
    t: f64,
) -> bool {
    let q: &mut CompilerResult = unsafe { &mut *q };

    if let Some(func) = &mut q.func {
        if func.count_states + func.count_params != ns || func.count_obs != nd {
            return false;
        }

        let du: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(du, nd) };
        let u: &[f64] = unsafe { std::slice::from_raw_parts(u, ns) };
        func.call_py(du, u, t);
        true
    } else {
        false
    }
}

#[no_mangle]
pub extern "C" fn finalize(p: *mut CompilerResult) {
    if !p.is_null() {
        let _ = unsafe { Box::from_raw(p) };
    }
}

#[no_mangle]
pub extern "C" fn info() -> *const c_char {
    let msg = c"lib 0.1";
    msg.as_ptr() as *const _
}

#[no_mangle]
pub extern "C" fn elem_at(
    v: *const f64,
    nv: usize,
    index: usize,
) -> f64 {
    let v: &[f64] = unsafe { std::slice::from_raw_parts(v, nv) };
    v[index]     
}


