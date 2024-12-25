use std::fs::File;
use std::io::{Write, BufWriter};
use std::time::Instant;

mod vector;
mod codegen;
mod register;
mod model;
mod code;
mod solvers;
mod amd;

use crate::model::*;
use crate::solvers::*;
// use crate::code::Intermediate;
use crate::amd::*;

fn solve(fun: &mut Function) {
    let u0 = fun.initial_states();
    let alg = Euler::new(0.02, 50);
    
    let now = Instant::now();
    // let alg = Euler::new(0.001, 10);        
    let sol = alg.solve(fun, &u0, 0.0..5000.0);
    println!("elapsed {:.1?}", now.elapsed());
    
    let fd = File::create("test.dat").expect("cannot open the file");
    let mut buf = BufWriter::new(fd);
    
    for row in &sol {    
        write!(&mut buf, "{}", row);
    };
}

fn main() {
    // test_codegen();
    let ml = CellModel::load("julia/test.json").unwrap();
    let mut prog = Program::new(&ml);
    ml.lower(&mut prog);
    //println!("{:#?}", prog);    
    //println!("ns = {}", prog.frame.count_states());
    //println!("np = {}", prog.frame.count_params());
    //println!("no = {}", prog.frame.count_obs());
    //println!("s = {}", prog.frame.first_state().unwrap());
    //println!("p = {}", prog.frame.first_param().unwrap());    
    
    let mut fun = Function::new(prog);
    solve(&mut fun);
}

