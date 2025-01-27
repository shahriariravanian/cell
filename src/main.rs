use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::time::Instant;

mod code;
mod model;
mod register;
mod runnable;
mod solvers;
mod utils;

mod amd;
mod arm;
mod interpreter;
mod rusty;
mod wasm;

use model::{CellModel, Program};
use runnable::{CompilerType, Runnable};
use solvers::*;

fn solve(r: &mut Runnable) {
    let u0 = r.initial_states();
    let p = r.params();
    let alg = Euler::new(0.02, 50);
    //let sol = alg.solve(r, u0.clone(), p.clone(), 0.0..1000.0);

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
        println!("use: cell [bytecode|amd|arm|native|wasm|rusty] model-file.json");
        std::process::exit(0);
    }

    let text = fs::read_to_string(args[2].as_str()).unwrap();
    let ml = CellModel::load(&text).unwrap();

    let (ty, reuse) = match args[1].as_str() {
        "bytecode" => (CompilerType::ByteCode, true),
        "arm" => (CompilerType::Arm, true),
        "amd" => (CompilerType::Amd, true),
        "native" => (CompilerType::Native, true),
        "wasm" => (CompilerType::Wasm, true),
        "rusty" => (CompilerType::Rusty, true),
        _ => {
            println!("compiler type should be one of bytecode, amd, arm, native, wasm, or. rusty");
            std::process::exit(0);
        }
    };

    let prog = Program::new(&ml, reuse);
    let mut r = Runnable::new(prog, ty);
    solve(&mut r);
}
