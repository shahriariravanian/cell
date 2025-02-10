use crate::model::Program;
use crate::utils::*;

use crate::amd::AmdCompiler;
use crate::arm::ArmCompiler;
use crate::interpreter::Interpreter;
#[cfg(feature = "rusty")]
use crate::rusty::RustyCompiler;
#[cfg(feature = "wasm")]
use crate::wasm::WasmCompiler;

#[derive(PartialEq)]
pub enum CompilerType {
    ByteCode,
    Native,
    Amd,
    Arm,
    #[cfg(feature = "wasm")]
    Wasm,
    #[cfg(feature = "rusty")]
    Rusty,
}

pub struct Runnable {
    pub prog: Program,
    pub compiled: Box<dyn Compiled>,
    pub first_state: usize,    
    pub first_param: usize,
    pub first_obs: usize,    
    pub first_diff: usize,
    pub count_states: usize,
    pub count_params: usize,
    pub count_obs: usize,
    pub count_diffs: usize,
    pub u0: Vec<f64>,
    pub p: Vec<f64>,
}

impl Runnable {
    pub fn new(prog: Program, ty: CompilerType) -> Runnable {
        let compiled: Box<dyn Compiled> = match ty {
            CompilerType::ByteCode => Box::new(Interpreter::new().compile(&prog)),
            #[cfg(feature = "wasm")]
            CompilerType::Wasm => Box::new(WasmCompiler::new().compile(&prog)),
            #[cfg(feature = "rusty")]
            CompilerType::Rusty => Box::new(RustyCompiler::new().compile(&prog)),
            CompilerType::Amd => Box::new(AmdCompiler::new().compile(&prog)),
            CompilerType::Arm => Box::new(ArmCompiler::new().compile(&prog)),
            #[cfg(target_arch = "x86_64")]
            CompilerType::Native => Box::new(AmdCompiler::new().compile(&prog)),
            #[cfg(target_arch = "aarch64")]
            CompilerType::Native => Box::new(ArmCompiler::new().compile(&prog)),
        };

        let first_state = prog.frame.first_state().unwrap();
        let first_param = prog.frame.first_param().unwrap();
        let first_obs = prog.frame.first_obs().unwrap();
        let first_diff = prog.frame.first_diff().unwrap();
        
        let count_states = prog.frame.count_states();        
        let count_params = prog.frame.count_params();
        let count_obs = prog.frame.count_obs();        
        let count_diffs = prog.frame.count_diffs();

        let mem = compiled.mem();
        let u0 = mem[first_state..first_state + count_states].to_vec();
        let p = mem[first_param..first_param + count_params].to_vec();

        Runnable {
            prog,
            compiled,
            first_state,
            first_param,
            first_obs,
            first_diff,
            count_states,            
            count_params,
            count_obs,            
            count_diffs,
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
    // call interface to Julia ODESolver
    fn call(&mut self, du: &mut [f64], u: &[f64], p: &[f64], t: f64) {
        {
            let mem = self.compiled.mem_mut();
            mem[self.first_state - 1] = t;
            let _ =
                &mut mem[self.first_state..self.first_state + self.count_states].copy_from_slice(u);
            let _ =
                &mut mem[self.first_param..self.first_param + self.count_params].copy_from_slice(p);
        }

        self.compiled.run();

        {
            let mem = self.compiled.mem();
            let _ = du.copy_from_slice(&mem[self.first_diff..self.first_diff + self.count_diffs]);
        }
    }
    
    // call interface to Python scipy ode solver    
    fn call_py(&mut self, du: &mut [f64], u: &[f64], t: f64) {
        {
            let mem = self.compiled.mem_mut();
            mem[self.first_state - 1] = t;
            let _ =
                &mut mem[self.first_state..self.first_state + self.count_states + self.count_params]
                    .copy_from_slice(u);
        }

        self.compiled.run();

        {
            let mem = self.compiled.mem();
            let _ = du.copy_from_slice(&mem[self.first_obs..self.first_obs + self.count_obs]);
        }
    }
}
