use std::fs;
use std::io::{Write, BufWriter};
use std::time::Instant;

mod utils;
mod vector;
mod register;
mod model;
mod code;
mod solvers;
mod amd;

use crate::utils::*;
use crate::model::*;
use crate::solvers::*;
use crate::amd::*;


pub struct Runnable {
    pub prog:           Program,
    pub mem:            Vec<f64>,    
    pub compiled:       Box<dyn Compiled>,    
    pub first_state:    usize,
    pub count_states:   usize,
    pub first_param:    usize,
    pub count_params:   usize,   
    pub u0:             Vec<f64>,     
}


impl Runnable {
    pub fn new(mut prog: Program) -> Runnable {
        // prog.calc_virtual_table();
        let mem = prog.frame.mem();
        let compiled = Box::new(NativeCompiler::new().compile(&prog));
                
        let first_state = prog.frame.first_state().unwrap();
        let count_states = prog.frame.count_states();
        let first_param = prog.frame.first_param().unwrap();
        let count_params = prog.frame.count_params();
        
        let u0 = mem[first_state..first_state+count_states].to_vec();              

        Runnable {
            prog,
            mem,
            compiled,
            first_state,
            count_states,
            first_param,
            count_params,
            u0,
        }
    }
    
    pub fn initial_states(&self) -> Vector {
        Vector(self.u0.clone())
    }
    
    pub fn params(&self) -> Vector {
        let p = self.mem[self.first_param..self.first_param+self.count_params].to_vec();
        Vector(p)
    }    
    
    pub fn run(&mut self) {        
        // self.prog.run(&mut self.mem[..], &self.prog.vt[..]);
        self.compiled.run(&mut self.mem[..]);
    }
    
    fn solve(&mut self) {
        let u0 = self.initial_states();
        let alg = Euler::new(0.02, 50);
        
        let now = Instant::now();
        // let alg = Euler::new(0.001, 10);        
        let sol = alg.solve(self, &u0, 0.0..5000.0);
        println!("elapsed {:.1?}", now.elapsed());
        
        let fd = fs::File::create("test.dat").expect("cannot open the file");
        let mut buf = BufWriter::new(fd);
        
        for row in &sol {    
            let _ = write!(&mut buf, "{}", row);
        };
    }
}

impl Callable for Runnable {
    fn call(&mut self, du: &mut Vector, u: &Vector, t: f64) {
        self.mem[self.first_state-1] = t;
        
        let p = &mut self.mem[self.first_state..self.first_state+self.count_states];
        p.copy_from_slice(u.as_slice());
        
        self.run();        
        
        let dp = &self.mem[self.first_state+self.count_states..self.first_state+2*self.count_states];
        du.as_mut_slice().copy_from_slice(dp);
    }
}


fn main() {
    // test_codegen();
    let text = fs::read_to_string("julia/test.json").unwrap();
    //let ml = CellModel::load("julia/test.json").unwrap();
    let ml = CellModel::load(&text).unwrap();
    let prog = Program::new(&ml);
    //println!("{:#?}", prog);    
    //println!("{}", prog.frame.as_json().unwrap());
    
    let mut runnable = Runnable::new(prog);
    runnable.solve();
}

