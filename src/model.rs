use serde::Deserialize;
use std::error::Error;

use crate::code::*;
use crate::register::*;

// lowers Expr and its constituents into a three-address_code format
pub trait Lower {
    fn lower(&self, prog: &mut Program) -> Reg;
}

// collects instructions and registers
#[derive(Debug)]
pub struct Program {
    pub code: Vec<Instruction>, // the list of instructions
    pub frame: Frame,           // memory (states, registers, constants, ...)
    pub ft: Vec<String>,        // function table (used to generate a virtual table)
}

impl Program {
    pub fn new(ml: &CellModel) -> Program {
        let mut frame = Frame::new();

        frame.alloc(RegType::Var(ml.iv.name.clone()), None);

        for v in &ml.states {
            frame.alloc(RegType::State(v.name.clone()), Some(v.val));
        }

        for v in &ml.states {
            frame.alloc(RegType::Diff(v.name.clone()), None);
        }

        for v in &ml.params {
            frame.alloc(RegType::Param(v.name.clone()), Some(v.val));
        }

        let mut prog = Program {
            code: Vec::new(),
            frame,
            ft: Vec::new(),
        };

        ml.lower(&mut prog);
        prog.code.push(Instruction::Nop);

        prog
    }

    // pushes a non-op into code
    // useful for debugging
    pub fn push(&mut self, s: Instruction) {
        self.code.push(s)
    }

    fn proc(&mut self, op: &str) -> Proc {
        let p = match self.ft.iter().position(|s| s == op) {
            Some(p) => p,
            None => {
                self.ft.push(op.to_string());
                self.ft.len() - 1
            }
        };
        Proc(p)
    }

    // pushes an Op into code and adjusts the virtual table accordingly
    pub fn push_unary(&mut self, op: &str, x: Reg, dst: Reg) {
        let p = self.proc(op);

        self.code.push(Instruction::Unary {
            op: op.to_string(),
            x,
            dst,
            p,
        })
    }

    pub fn push_binary(&mut self, op: &str, x: Reg, y: Reg, dst: Reg) {
        let p = self.proc(op);

        self.code.push(Instruction::Binary {
            op: op.to_string(),
            x,
            y,
            dst,
            p,
        })
    }

    pub fn push_ifelse(&mut self, x: Reg, y: Reg, z: Reg, dst: Reg) {
        self.code.push(Instruction::IfElse { x, y, z, dst })
    }

    // allocates a constant register
    pub fn alloc_const(&mut self, val: f64) -> Reg {
        self.frame.alloc(RegType::Const, Some(val))
    }

    // allocates a temporary register
    pub fn alloc_temp(&mut self) -> Reg {
        self.frame.alloc(RegType::Temp, None)
    }

    // allocates an obeservable register
    pub fn alloc_obs(&mut self, name: &str) -> Reg {
        self.frame.alloc(RegType::Obs(name.to_string()), None)
    }

    pub fn free(&mut self, r: Reg) {
        self.frame.free(r)
    }

    pub fn reg(&self, name: &str) -> Reg {
        for reg_type in [RegType::State, RegType::Param, RegType::Obs, RegType::Var] {
            if let Some(r) = self.frame.find(&reg_type(name.to_string())) {
                return r;
            }
        }
        panic!("cannot find reg by name");
    }

    pub fn reg_diff(&self, name: &str) -> Reg {
        if let Some(r) = self.frame.find(&RegType::Diff(name.to_string())) {
            return r;
        }
        panic!("cannot find diff by name");
    }

    pub fn virtual_table(&self) -> Vec<fn(f64, f64) -> f64> {
        let vt: Vec<fn(f64, f64) -> f64> = self.ft.iter().map(|s| Code::from_str(s)).collect();
        vt
    }
}

// A defined (state or param) variable
#[derive(Debug, Clone, Deserialize)]
pub struct Variable {
    pub name: String,
    pub val: f64,
}

impl Lower for Variable {
    fn lower(&self, prog: &mut Program) -> Reg {
        prog.reg(&self.name)
    }
}

// Expr tree
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Expr {
    Tree { op: String, args: Vec<Expr> },
    Const { val: f64 },
    Var { name: String },
}

impl Expr {
    // extracts the differentiated variable from the lhs of a diff eq
    pub fn diff_var(&self) -> Option<String> {
        if let Expr::Tree { args, op } = self {
            if op != "Differential" {
                return None;
            }
            if let Expr::Var { name } = &args[0] {
                return Some(name.clone());
            }
        };
        None
    }

    // extracts the regular variable from the lhs of an observable eq
    pub fn var(&self) -> Option<String> {
        if let Expr::Var { name } = self {
            return Some(name.clone());
        };
        None
    }

    fn lower_unary(&self, prog: &mut Program, op: &str, args: &Vec<Expr>) -> Reg {
        let x = args[0].lower(prog);
        let dst = prog.alloc_temp();
        prog.push_unary(op, x, dst);
        prog.free(x);
        dst
    }

    fn lower_binary(&self, prog: &mut Program, op: &str, args: &Vec<Expr>) -> Reg {
        let x = args[0].lower(prog);
        let y = args[1].lower(prog);
        let dst = prog.alloc_temp();

        if op == "times" && x == Reg(2) {
            prog.push_unary("neg", y, dst);
        } else if op == "times" && y == Reg(2) {
            prog.push_unary("neg", x, dst);
        } else {
            prog.push_binary(op, x, y, dst);
        }

        prog.free(x);
        prog.free(y);
        dst
    }

    fn lower_ternary(&self, prog: &mut Program, op: &str, args: &Vec<Expr>) -> Reg {
        if op != "ifelse" {
            return self.lower_poly(prog, op, args);
        }

        let x = args[0].lower(prog);
        let y1 = args[1].lower(prog);
        let y2 = args[2].lower(prog);
        let dst = prog.alloc_temp();

        prog.push_ifelse(x, y1, y2, dst);

        prog.free(x);
        prog.free(y1);
        prog.free(y2);

        dst
    }

    fn lower_poly(&self, prog: &mut Program, op: &str, args: &Vec<Expr>) -> Reg {
        if !(op == "plus" || op == "times") {
            panic!("missing op: {}", op);
        }

        let mut x = args[0].lower(prog);
        for i in 1..args.len() {
            let y = args[i].lower(prog);
            let dst = prog.alloc_temp();
            prog.push_binary(op, x, y, dst);
            prog.free(x);
            x = dst;
        }

        x
    }
}

impl Lower for Expr {
    fn lower(&self, prog: &mut Program) -> Reg {
        match self {
            Expr::Const { val } => {
                // Optimization!
                // we assume that the value of Reg(0) is 0.0, Reg(1) is 1,
                // and Reg(2) is -1
                if *val == 0.0 {
                    Reg(0)
                } else if *val == 1.0 {
                    Reg(1)
                } else if *val == -1.0 {
                    Reg(2)
                } else {
                    // let dst = prog.alloc_temp();
                    let dst = prog.alloc_const(*val);
                    prog.push(Instruction::Num { val: *val, dst }); // not needed for code generation, useful for debugging
                    dst
                }
            }
            Expr::Var { name } => {
                // Technically, this is not necessary but having Instruction::Var in the code
                // is helpful for debugging
                let dst = prog.reg(name);
                prog.push(Instruction::Var {
                    name: name.clone(),
                    reg: dst,
                });
                dst
            }
            Expr::Tree { op, args } => match args.len() {
                1 => self.lower_unary(prog, &op, &args),
                2 => self.lower_binary(prog, &op, &args),
                3 => self.lower_ternary(prog, &op, &args),
                _ => self.lower_poly(prog, &op, &args),
            },
        }
    }
}

// abstracts equation lhs ~ rhs
#[derive(Debug, Clone, Deserialize)]
pub struct Equation {
    pub lhs: Expr,
    pub rhs: Expr,
}

impl Lower for Equation {
    fn lower(&self, prog: &mut Program) -> Reg {
        let src = self.rhs.lower(prog);

        let dst = if let Some(var) = self.lhs.diff_var() {
            prog.reg_diff(&var)
        } else if let Some(var) = self.lhs.var() {
            prog.alloc_obs(&var)
        } else {
            panic!("undefined diff variable");
        };

        prog.push_unary("mov", src, dst);
        Reg(0)
    }
}

// loads from a JSON CellModel file
#[derive(Debug, Clone, Deserialize)]
pub struct CellModel {
    pub iv: Variable,
    pub params: Vec<Variable>,
    pub states: Vec<Variable>,
    pub algs: Vec<Equation>,
    pub odes: Vec<Equation>,
    pub obs: Vec<Equation>,
}

impl CellModel {
    pub fn load(text: &str) -> Result<CellModel, Box<dyn Error>> {
        Ok(serde_json::from_str(text)?)
    }
}

impl Lower for CellModel {
    fn lower(&self, prog: &mut Program) -> Reg {
        for eq in &self.obs {
            eq.lower(prog);
        }

        for eq in &self.odes {
            eq.lower(prog);
        }

        Reg(0)
    }
}
