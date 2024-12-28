use std::fs;
use std::io::{Write, BufWriter};
use std::time::Instant;

mod utils;
mod vector;
mod codegen;
mod register;
mod model;
mod code;
mod solvers;
mod amd;

use crate::model::*;
use crate::solvers::*;
use crate::amd::*;

fn solve(fun: &mut Function) {
    let u0 = fun.initial_states();
    let alg = Euler::new(0.02, 50);
    
    let now = Instant::now();
    // let alg = Euler::new(0.001, 10);        
    let sol = alg.solve(fun, &u0, 0.0..5000.0);
    println!("elapsed {:.1?}", now.elapsed());
    
    let fd = fs::File::create("test.dat").expect("cannot open the file");
    let mut buf = BufWriter::new(fd);
    
    for row in &sol {    
        write!(&mut buf, "{}", row);
    };
}

fn main() {
    // test_codegen();
    let text = fs::read_to_string("julia/test.json").unwrap();
    //let ml = CellModel::load("julia/test.json").unwrap();
    let ml = CellModel::load(&text).unwrap();
    let mut prog = Program::new(&ml);
    //println!("{:#?}", prog);    
    //println!("{}", prog.frame.as_json().unwrap());
    
    let mut fun = Function::new(prog);
    solve(&mut fun);
}

