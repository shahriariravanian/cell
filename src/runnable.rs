use std::fs;
use std::io::{BufWriter, Write};
use std::time::Instant;

use crate::amd::NativeCompiler;
use crate::interpreter::Interpreter;
use crate::model::{CellModel, Program};
use crate::solvers::*;
use crate::utils::*;
use crate::wasm::WasmCompiler;

pub enum CompilerType {
    ByteCode,
    Native,
    Wasm,
}

pub struct Runnable {
    pub prog: Program,
    pub compiled: Box<dyn Compiled>,
    pub first_state: usize,
    pub count_states: usize,
    pub first_param: usize,
    pub count_params: usize,
    pub u0: Vec<f64>,
    pub p: Vec<f64>,
}

impl Runnable {
    pub fn new(mut prog: Program, ty: CompilerType) -> Runnable {
        let mut compiled: Box<dyn Compiled> = match ty {
            CompilerType::ByteCode => Box::new(Interpreter::new().compile(&prog)),
            CompilerType::Native => Box::new(NativeCompiler::new().compile(&prog)),
            CompilerType::Wasm => Box::new(WasmCompiler::new().compile(&prog)),
        };

        let first_state = prog.frame.first_state().unwrap();
        let count_states = prog.frame.count_states();
        let first_param = prog.frame.first_param().unwrap();
        let count_params = prog.frame.count_params();

        let mem = compiled.mem();
        let u0 = mem[first_state..first_state + count_states].to_vec();
        let p = mem[first_param..first_param + count_params].to_vec();

        Runnable {
            prog,
            compiled,
            first_state,
            count_states,
            first_param,
            count_params,
            u0,
            p,
        }
    }

    pub fn initial_states(&self) -> Vec<f64> {
        self.u0.clone()
    }

    pub fn params(&self) -> Vec<f64> {
        self.p.clone()
    }
}

impl Callable for Runnable {
    fn call(&mut self, du: &mut [f64], u: &[f64], p: &[f64], t: f64) {
        {
            let mut mem = self.compiled.mem_mut();
            mem[self.first_state - 1] = t;
            &mut mem[self.first_state..self.first_state + self.count_states].copy_from_slice(u);
            &mut mem[self.first_param..self.first_param + self.count_params].copy_from_slice(p);
        }

        self.compiled.run();

        {
            let mem = self.compiled.mem();
            du.copy_from_slice(
                &mem[self.first_state + self.count_states
                    ..self.first_state + 2 * self.count_states],
            );
        }
    }
}
