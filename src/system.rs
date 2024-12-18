use libm;

use serde_json::Value;
use serde::Deserialize;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::collections::{HashMap, HashSet};

use crate::register::*;
use crate::code::*;


#[derive(Debug)]
pub struct Program {
    pub prog:   Vec<Instruction>,
    pub frame:  Frame
}

impl Program {
    pub fn new(sys: &System) -> Program {
        let mut frame = Frame::new();

        frame.alloc(RegType::State(sys.iv.name.clone()), None);
        
        for v in &sys.states {
            frame.alloc(RegType::State(v.name.clone()), Some(v.val));
        }
        
        for v in &sys.states {
            frame.alloc(RegType::Diff(v.name.clone()), None);
        }
        
        for v in &sys.params {
            frame.alloc(RegType::Param(v.name.clone()), Some(v.val));
        }                
    
        Program {
            prog:   Vec::new(),
            frame
        }
    }
    
    pub fn push(&mut self, s: Instruction) {
        self.prog.push(s)
    }
    
    pub fn alloc_temp(&mut self) -> Reg {
        self.frame.alloc(RegType::Temp, None)
    }
    
    pub fn alloc_obs(&mut self, name: &str) -> Reg {
        self.frame.alloc(RegType::Obs(name.to_string()), None)
    }
    
    pub fn free(&mut self, r: Reg) {
        self.frame.free(r)
    }
    
    pub fn reg(&self, name: &str) -> Reg {        
        if let Some(r) = self.frame.find(&RegType::State(name.to_string())) {
            return r;
        }
        if let Some(r) = self.frame.find(&RegType::Param(name.to_string())) {
            return r;
        }
        if let Some(r) = self.frame.find(&RegType::Obs(name.to_string())) {
            return r;
        }
        panic!("cannot find reg by name");
    }
    
    pub fn reg_diff(&self, name: &str) -> Reg {
        if let Some(r) = self.frame.find(&RegType::Diff(name.to_string())) {
            return r;
        }
        panic!("cannot find diff by name");
    }
}

pub trait Lower {
    fn lower(&self, prog: &mut Program) -> Reg;
}

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


#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Expr {
    Tree { op: String, args: Vec<Expr> },
    Const { val: f64 },
    Var { name: String }    
}

impl Expr {
    pub fn diff_var(&self) -> Option<String> {
        if let Expr::Tree{ args, op } = self {
            if op != "Differential" {
                return None
            }
            if let Expr::Var{ name } = &args[0] {
                return Some(name.clone())
            } 
        };
        None
    }
    
    pub fn var(&self) -> Option<String> {
        if let Expr::Var{ name } = self {
            return Some(name.clone())         
        };
        None
    }
    
    fn lower_unary(&self, prog: &mut Program, op: &str, args: &Vec<Expr>) -> Reg { 
        let x = args[0].lower(prog);
        let y = Reg(0);
        let dst = prog.alloc_temp();            
        let f = match op {
            "minus" => Code::neg,
            "sin"   => Code::sin,
            "cos"   => Code::cos,
            "tan"   => Code::tan,
            "csc"   => Code::csc,
            "sec"   => Code::sec,
            "cot"   => Code::cot,
            "arcsin"  => Code::asin,
            "arccos"  => Code::acos,
            "arctan"  => Code::atan,            
            "exp"   => Code::exp,
            "log"   => Code::log,
            "ln"    => Code::ln,
            "root"  => Code::root,
            _       => { panic!("missing op: {}", op); }
        };
        
        prog.push(Instruction::Op { op: op.to_string(), f, x, y, dst });                
        prog.free(x);
        dst
    }
    
    fn lower_binary(&self, prog: &mut Program, op: &str, args: &Vec<Expr>) -> Reg { 
        let x = args[0].lower(prog);
        let y = args[1].lower(prog);
        let dst = prog.alloc_temp();        
        let f = match op {
            "plus"      => Code::plus,
            "minus"     => Code::minus,
            "times"     => Code::times,           
            "divide"    => Code::divide,
            "power"     => Code::power,
            "rem"       => Code::rem,
            "gt"        => Code::gt, 
            "geq"       => Code::geq,
            "lt"        => Code::lt,
            "leq"       => Code::leq,
            "eq"        => Code::eq,
            "neq"       => Code::neq,
            "and"       => Code::and,
            "or"        => Code::or,
            "xor"       => Code::xor,
            _           => { panic!("missing op: {}", op); }
        };
        
        prog.push(Instruction::Op { op: op.to_string(), f, x, y, dst });        
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
        let t1 = prog.alloc_temp();
        let t2 = prog.alloc_temp();    
        let dst = prog.alloc_temp();     
          
        prog.push(Instruction::Op { op: "if_pos".to_string(), f: Code::if_pos, x, y: y1, dst: t1 });
        prog.push(Instruction::Op { op: "if_neg".to_string(), f: Code::if_neg, x, y: y2, dst: t2 });
        prog.push(Instruction::Op { op: "plus".to_string(), f: Code::plus, x: t1, y: t2, dst });
        
        prog.free(x);
        prog.free(y1);
        prog.free(y2);
        prog.free(t1);
        prog.free(t2);
        
        dst
    }
    
    fn lower_poly(&self, prog: &mut Program, op: &str, args: &Vec<Expr>) -> Reg { 
        let f = match op {
            "plus"      => Code::plus,
            "times"     => Code::times,
            _           => { panic!("missing op: {}", op); }
        };
        
        let mut x = args[0].lower(prog);
        for i in 1..args.len() {
            let y = args[i].lower(prog);
            let dst = prog.alloc_temp();
            prog.push(Instruction::Op { op: op.to_string(), f, x, y, dst });
            prog.free(x);
            x = dst;
        };
        
        x
    }
}

impl Lower for Expr {
    fn lower(&self, prog: &mut Program) -> Reg {
        match self {
            Expr::Const { val } => {
                let dst = prog.alloc_temp();
                prog.push(Instruction::Num { val: *val, dst });
                dst
            },
            Expr::Var { name } => {
                let dst = prog.reg(name);
                prog.push(Instruction::Var { name: name.clone(), reg: dst });
                dst                
            },
            Expr::Tree { op, args } => {
                match args.len() {
                    1 => self.lower_unary(prog, &op, &args),
                    2 => self.lower_binary(prog, &op, &args),
                    3 => self.lower_ternary(prog, &op, &args),
                    _ => self.lower_poly(prog, &op, &args),
                }    
            }
        }
    }    
}

#[derive(Debug, Clone, Deserialize)]
pub struct Equation {
    pub lhs: Expr,
    pub rhs: Expr
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
        
        prog.push(Instruction::Op { op: "mov".to_string(), f: Code::mov, x: src, y: Reg(0), dst });
        Reg(0)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct System {
    pub iv: Variable,
    pub params: Vec<Variable>,
    pub states: Vec<Variable>,
    pub algs: Vec<Equation>,
    pub odes: Vec<Equation>,
    pub obs: Vec<Equation>,
}

impl System {
    pub fn reg_base(&self) -> Reg {
        //Reg(2 + 2 * self.states.len() + self.params.len() + self.obs.len())
        Reg(2 + 2 * self.states.len() + self.params.len())
    }
}

impl Lower for System {
    fn lower(&self, prog: &mut Program) -> Reg {
        for eq in &self.obs {
            eq.lower(prog);
        };
        
        for eq in &self.odes {
            eq.lower(prog);
        };
        
        Reg(0)
    }
}

pub fn load_system<P: AsRef<Path>>(path: P) -> Result<System, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let sys = serde_json::from_reader(reader)?;
    Ok(sys)
}
