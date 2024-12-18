mod codegen;
mod register;
mod system;

use crate::codegen::test_codegen;
use crate::system::*;

fn main() {
    // test_codegen();
    let sys = load_system("julia/test.json").unwrap();
    //println!("{:#?}", sys);
    let mut prog = Program::new(&sys);   
    sys.lower(&mut prog);
    println!("{:#?}", prog.prog);    
}

