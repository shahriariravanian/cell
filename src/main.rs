use std::fs::File;
use std::io::{Write, BufWriter};

mod codegen;
mod register;
mod system;
mod code;

use crate::codegen::test_codegen;
use crate::system::*;

fn solve(prog: &Program) {
    let mut fun = Function::new(&prog);
    let mut u = fun.initial_states();
    let mut du = u.clone();
    
    let mut fd = File::create("test.dat").expect("cannot open the file");
    let mut buf = BufWriter::new(fd);
    
    for i in 0..5000 {    
        writeln!(&mut buf, "{}\t{}\t{}", u[0], u[1], u[2]);
        fun.call(&mut du, &u, 0.0);
        for j in 0..3 {
            u[j] += 0.01 * du[j];
        }
    }
}

fn main() {
    // test_codegen();
    let sys = load_system("julia/test.json").unwrap();
    //println!("{:#?}", sys);
    let mut prog = Program::new(&sys);   
    sys.lower(&mut prog);
    println!("{:#?}", prog);    
    println!("ns = {}", prog.frame.count_states());
    println!("np = {}", prog.frame.count_params());
    println!("no = {}", prog.frame.count_obs());
    println!("s = {}", prog.frame.first_state().unwrap());
    println!("p = {}", prog.frame.first_param().unwrap());
 
    solve(&prog);
}

