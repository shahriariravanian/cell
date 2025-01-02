use std::fs;
use std::io::{BufWriter, Write};
use std::time::Instant;

mod amd;
mod code;
mod interpreter;
mod model;
mod register;
mod runnable;
mod solvers;
mod utils;
mod vector;
mod wasm;

use crate::model::{CellModel, Program};
use crate::runnable::{CompilerType, Runnable};
use crate::solvers::*;
use crate::utils::*;

use crate::wasm::*;

fn solve(r: &mut Runnable) {
    let u0 = r.initial_states();
    let p = r.params();
    let alg = Euler::new(0.02, 50);

    let now = Instant::now();
    // let alg = Euler::new(0.001, 10);
    let sol = alg.solve(r, u0, p, 0.0..2000.0);
    println!("elapsed {:.1?}", now.elapsed());

    let fd = fs::File::create("test.dat").expect("cannot open the file");
    let mut buf = BufWriter::new(fd);

    for row in &sol {
        let _ = write!(&mut buf, "{}", row);
    }
}

fn main() {
    // test_codegen();
    let text = fs::read_to_string("julia/test.json").unwrap();
    let ml = CellModel::load(&text).unwrap();
    let prog = Program::new(&ml);

    //let mut wasm = WasmCompiler::new().compile(&prog);
    //wasm.imports();

    let mut r = Runnable::new(prog, CompilerType::Wasm);
    solve(&mut r);
}
