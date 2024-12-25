use serde::Deserialize;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use std::rc::Rc;
use std::cell::RefCell;

use crate::register::*;
use crate::code::*;
use crate::vector::*;
use crate::solvers::*;
use crate::amd::*;

// lowers Expr and its constituents into a three-address_code format
pub trait Lower {
    fn lower(&self, prog: &mut Program) -> Reg;
}


// collects instructions and registers
#[derive(Debug)]
pub struct Program {
    pub code:   Vec<Instruction>,       // the list of instructions
    pub frame:  Frame,                  // memory (states, registers, constants, ...)
    pub vt:     Vec<fn(f64,f64)->f64>,  // virtual table
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
    
        Program {
            code:   Vec::new(),  
            frame,
            vt:     Vec::new(),
        }
    }
        
    // pushes a non-op into code
    // useful for debugging
    pub fn push(&mut self, s: Instruction) {
        self.code.push(s)
    }
        
    // pushes an Op into code and adjusts the virtual table accordingly
    pub fn push_op(&mut self, op: &str, f: fn(f64,f64)->f64, x: Reg, y: Reg, dst: Reg) {
        let p = match self.vt.iter().position(|&g| f == g) {
            Some(p) => p,
            None => {                
                self.vt.push(f);
                self.vt.len() - 1
            }
        };
        self.code.push(Instruction::Op { op: op.to_string(), x, y, dst, p: Proc(p) })
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

    // runs the program using a bytecode interpreter
    pub fn run(&self, mem: &mut Vec<f64>, vt: &Vec<fn (f64, f64) -> f64>) {    
        for c in self.code.iter()  {
            match c {
                Instruction::Num {..} => {},    // Num and Var do not generate any code 
                Instruction::Var {..} => {},    // They are mainly for debugging
                Instruction::Op {p, x, y, dst, ..} => { 
                    mem[dst.0] = self.vt[p.0](mem[x.0], mem[y.0]);
                }
            }
        }
    }
}


// abstracts a function passed to an ODE solver
#[derive(Debug)]
pub struct Function {
    pub prog:           Program,
    pub mem:            Vec<f64>,    
    pub compiled:       Compiled,    
    pub first_state:    usize,
    pub count_states:   usize,
    pub first_param:    usize,
    pub count_params:   usize,   
    pub u0:             Vec<f64>,     
}

impl Function {
    pub fn new(prog: Program) -> Function {
        // Function consumes Program
        let mem = prog.frame.mem();
        let compiled = Amd64::new().compile(&prog);
                
        let first_state = prog.frame.first_state().unwrap();
        let count_states = prog.frame.count_states();
        let first_param = prog.frame.first_param().unwrap();
        let count_params = prog.frame.count_params();
        
        let u0 = mem[first_state..first_state+count_states].to_vec();              

        Function {
            prog,
            mem,
            compiled,
            first_state,
            count_states,
            first_param,
            count_params,
            u0,
        }
    }
    
    pub fn initial_states(&self) -> Vector {
        Vector(self.u0.clone())
    }
    
    pub fn params(&self) -> Vector {
        let p = self.mem[self.first_param..self.first_param+self.count_params].to_vec();
        Vector(p)
    }    
    
    pub fn run(&mut self) {        
        // self.prog.run(&mut self.mem, &self.prog.vt);
        self.compiled.run(&mut self.mem, &self.prog.vt);
    }
}

impl Callable for Function {
    fn call(&mut self, du: &mut Vector, u: &Vector, t: f64) {
        self.mem[3] = t;    // TODO: hardcoded iv address        
        
        let p = &mut self.mem[self.first_state..self.first_state+self.count_states];
        p.copy_from_slice(u.as_slice());
        
        self.run();        
        
        let dp = &self.mem[self.first_state+self.count_states..self.first_state+2*self.count_states];
        du.as_mut_slice().copy_from_slice(dp);
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
    Var { name: String }    
}

impl Expr {
    // extracts the differentiated variable from the lhs of a diff eq
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

    // extracts the regular variable from the lhs of an observable eq    
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

        prog.push_op(op, f, x, y, dst);                
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

        prog.push_op(op, f, x, y, dst);
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
        
        prog.push_op("if_pos", Code::if_pos, x, y1, t1);        
        prog.push_op("if_neg", Code::if_neg, x, y2, t2);
        prog.push_op("plus", Code::plus, t1, t2, dst);
        
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
            prog.push_op(op, f, x, y, dst);
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
                // Optimization!
                // we assume that the value of Reg(0) is 0.0, Reg(1) is 1, 
                // and Reg(2) is -1
                if *val == 0.0 {
                    Reg(0)
                } else if *val == 1.0 {
                    Reg(1)
                } else if *val == -1.0 {
                    Reg(2)
                } else{
                    // let dst = prog.alloc_temp();
                    // prog.push(Instruction::Num { val: *val, dst });
                    let dst = prog.alloc_const(*val);                    
                    prog.push(Instruction::Num { val: *val, dst }); // not needed for code generation, useful for debugging                    
                    dst
                }
            },
            Expr::Var { name } => {
                // Technically, this is not necessary but having Instruction::Var in the code
                // is helpful for debugging
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

// abstracts equation lhs ~ rhs
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
        
        prog.push_op("mov", Code::mov, src, Reg(0), dst);
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
    pub fn load<P: AsRef<Path>>(path: P) -> Result<CellModel, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let ml = serde_json::from_reader(reader)?;
        Ok(ml)
    }
}
    
impl Lower for CellModel {    
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

