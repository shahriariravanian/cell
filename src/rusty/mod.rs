use std::fs;
use std::io::{BufWriter, Write};

use crate::code::*;
use crate::model::Program;
use crate::register::Word;
use crate::utils::*;

//mod func;

//use func::*;

#[derive(Debug)]
pub struct RustyCompiler {
    stack: Vec<String>,
}

impl RustyCompiler {
    pub fn new() -> RustyCompiler {
        Self { stack: Vec::new() }
    }

    fn unary(op: &str, x: String) -> String {
        match op {
            "neg" => format!("-({})", x),
            "sin" => format!("f64::sin({})", x),
            "cos" => format!("f64::cos({})", x),
            "tan" => format!("f64::tan({})", x),
            "csc" => format!("(1.0 / f64::sin({}))", x),
            "sec" => format!("(1.0 / f64::cos({}))", x),
            "cot" => format!("(1.0 / f64::tan({}))", x),
            "arcsin" => format!("f64::asin({})", x),
            "arccos" => format!("f64::acos({})", x),
            "arctan" => format!("f64::atan({})", x),
            "exp" => format!("f64::exp({})", x),
            "ln" => format!("f64::ln({})", x),
            "log" => format!("f64::log({}, 10.0)", x),
            "root" => format!("f64::sqrt({})", x),
            _ => {
                let msg = format!("unary op_code {} not found", op);
                panic!("{}", msg);
            }
        }
    }

    fn binary(op: &str, x: String, y: String) -> String {
        match op {
            "plus" => format!("({}) + ({})", x, y),
            "minus" => format!("({}) - ({})", x, y),
            "times" => format!("({}) * ({})", x, y),
            "divide" => format!("({}) / ({})", x, y),
            "rem" => format!("({}) % ({})", x, y),
            "gt" => format!("({}) > ({})", x, y),
            "geq" => format!("({}) >= ({})", x, y),
            "lt" => format!("({}) < ({})", x, y),
            "leq" => format!("({}) <= ({})", x, y),
            "eq" => format!("({}) == ({})", x, y),
            "neq" => format!("({}) != ({})", x, y),
            "and" => format!("({}) & ({})", x, y),
            "or" => format!("({}) ! ({})", x, y),
            "xor" => format!("({}) ^ ({})", x, y),
            "power" => format!("f64::powf({}, {})", x, y),
            _ => {
                let msg = format!("binary op_code {} not found", op);
                panic!("{}", msg);
            }
        }
    }

    fn compose(&mut self, prog: &Program) {
        for c in prog.code.iter() {
            match c {
                Instruction::Unary { op, .. } => {
                    let s = if op == "mov" {
                        let rhs = self.stack.pop().unwrap();
                        let lhs = self.stack.pop().unwrap();
                        if lhs.starts_with("t_") {
                            format!("let {} = {}", lhs, rhs)
                        } else {
                            format!("{} = {}", lhs, rhs)
                        }
                    } else {
                        let x = self.stack.pop().unwrap();
                        Self::unary(op, x)
                    };
                    self.stack.push(s);
                }
                Instruction::Binary { op, .. } => {
                    let y = self.stack.pop().unwrap();
                    let x = self.stack.pop().unwrap();
                    let s = Self::binary(op, x, y);
                    self.stack.push(s);
                }
                Instruction::IfElse { .. } => {
                    let cond = self.stack.pop().unwrap();
                    let x2 = self.stack.pop().unwrap();
                    let x1 = self.stack.pop().unwrap();
                    let s = format!("(if {} {{{}}} else {{{}}})", cond, x1, x2);
                    self.stack.push(s);
                }
                Instruction::Eq { dst } => {
                    if prog.frame.is_obs(&dst) {
                        self.stack.push(format!("t_{}", dst.0));
                    } else {
                        self.stack.push(format!("mem[{}]", dst.0));
                    }
                }
                Instruction::Num { val, .. } => {
                    self.stack.push(format!("({} as f64)", val));
                }
                Instruction::Var { reg, .. } => {
                    if prog.frame.is_obs(&reg) {
                        self.stack.push(format!("t_{}", reg.0));
                    } else {
                        self.stack.push(format!("mem[{}]", reg.0));
                    }
                }
                _ => {}
            }
        }
    }
}

impl Compiler<RustyCode> for RustyCompiler {
    fn compile(&mut self, prog: &Program) -> RustyCode {
        self.compose(prog);

        let fd = fs::File::create("src/rusty/func.rs").expect("cannot create func.rs");
        let mut buf = BufWriter::new(fd);

        //let _ = writeln!(&mut buf, "use crate::code::BinaryFunc;");
        let _ = writeln!(&mut buf, "#![allow(unused_parens)]");
        let _ = writeln!(&mut buf, "pub fn func(mem: &mut [f64]) {{");

        for sm in self.stack.iter() {
            let _ = writeln!(&mut buf, "\t{};", sm);
        }

        let _ = writeln!(&mut buf, "}}");

        RustyCode::new(prog.frame.mem())
    }
}

pub struct RustyCode {
    _mem: Vec<f64>,
}

impl RustyCode {
    fn new(_mem: Vec<f64>) -> RustyCode {
        RustyCode { _mem }
    }
}

impl Compiled for RustyCode {
    fn run(&mut self) {
        //func(&mut self._mem);
    }

    #[inline]
    fn mem(&self) -> &[f64] {
        &self._mem[..]
    }

    #[inline]
    fn mem_mut(&mut self) -> &mut [f64] {
        &mut self._mem[..]
    }
}
