use std::env;
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
//use crate::utils::*;

//use crate::wasm::*;

fn solve(r: &mut Runnable) {
    let u0 = r.initial_states();
    let p = r.params();
    let alg = Euler::new(0.02, 50);

    let now = Instant::now();
    // let alg = Euler::new(0.001, 10);
    let sol = alg.solve(r, u0, p, 0.0..5000.0);
    println!("elapsed {:.1?}", now.elapsed());

    let fd = fs::File::create("test.dat").expect("cannot open the file");
    let mut buf = BufWriter::new(fd);

    for row in &sol {
        let _ = write!(&mut buf, "{}", row);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("use: cell [bytecode|native|wasm] model-file.json");
        std::process::exit(0);
    }

    let text = fs::read_to_string(args[2].as_str()).unwrap();
    let ml = CellModel::load(&text).unwrap();
    let prog = Program::new(&ml);
    
    println!("{:#?}", &prog);

    //let mut wasm = WasmCompiler::new().compile(&prog);
    //wasm.imports();

    let ty = match args[1].as_str() {
        "bytecode" => CompilerType::ByteCode,
        "native" => CompilerType::Native,
        "wasm" => CompilerType::Wasm,
        _ => {
            println!("compiler type should be one of bytecode, native, or wasm");
            std::process::exit(0);
        }
    };

    let mut r = Runnable::new(prog, ty);
    solve(&mut r);
}
