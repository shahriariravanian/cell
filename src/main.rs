mod codegen;
mod system;

use crate::codegen::test_codegen;
use crate::system::*;
use crate::system::Lower;

fn main() {
    // test_codegen();
    let sys = load_system("julia/test.json").unwrap();
    //println!("{:#?}", sys);
    let c = sys.count_const();
    println!("{}", c);
    let mut prog = Program::new(&sys);   
    sys.lower(&mut prog);
    println!("{:#?}", prog.prog);    
}

