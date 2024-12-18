use libm;

use serde_json::Value;
use serde::Deserialize;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::collections::{HashMap, HashSet};

use crate::register::Reg;

type Tac = fn (f64, f64) -> f64;

pub enum Instruction {
    Op{
        op: String, 
        x: Reg,
        y: Reg,
        dst: Reg,
        f: Tac
    },
    Num {
        val: f64,
        dst: Reg
    },
    Var {
        name: String,
        reg: Reg
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Instruction::Op { op, x, y, dst, .. } => write!(f, "r{:<6}← r{} {} r{}", dst.0, x.0, op, y.0),
            Instruction::Num { val, dst } => write!(f, "r{:<6}= {}", dst.0, val),
            Instruction::Var { name, reg } => write!(f, "r{:<6}:: {}", reg.0, name),
        }
    }
}

impl std::fmt::Debug for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

pub struct Code {}

impl Code {
    pub fn mov(x: f64, _y: f64) -> f64 {
        x
    }

    pub fn plus(x: f64, y: f64) -> f64 {
        x + y
    }
    
    pub fn minus(x: f64, y: f64) -> f64 {
        x - y
    }
    
    pub fn neg(x: f64, y: f64) -> f64 {
        -x
    }
    
    pub fn times(x: f64, y: f64) -> f64 {
        x * y
    }
    
    pub fn divide(x: f64, y: f64) -> f64 {
        x / y
    }
    
    pub fn rem(x: f64, y: f64) -> f64 {
        x % y
    }
    
    pub fn power(x: f64, y: f64) -> f64 {
        x.powf(y)
    }    
    
    pub fn gt(x: f64, y: f64) -> f64 {
        if x > y { 1.0 } else { -1.0 }
    }
    
    pub fn geq(x: f64, y: f64) -> f64 {
        if x >= y { 1.0 } else { -1.0 }
    }
    
    pub fn lt(x: f64, y: f64) -> f64 {
        if x < y { 1.0 } else { -1.0 }
    }
    
    pub fn leq(x: f64, y: f64) -> f64 {
        if x <= y { 1.0 } else { -1.0 }
    }
    
    pub fn eq(x: f64, y: f64) -> f64 {
        if x == y { 1.0 } else { -1.0 }
    }
    
    pub fn neq(x: f64, y: f64) -> f64 {
        if x != y { 1.0 } else { -1.0 }
    }
    
    pub fn and(x: f64, y: f64) -> f64 {
        if x > 0.0 && y > 0.0 { 1.0 } else { -1.0 }
    }
    
    pub fn or(x: f64, y: f64) -> f64 {
        if x > 0.0 || y > 0.0 { 1.0 } else { -1.0 }
    }
    
    pub fn xor(x: f64, y: f64) -> f64 {
        if x * y < 0.0 { 1.0 } else { -1.0 }
    }
    
    pub fn if_pos(x: f64, y: f64) -> f64 {
        if x > 0.0 { y } else { 0.0 }
    }
    
    pub fn if_neg(x: f64, y: f64) -> f64 {
        if x < 0.0 { y } else { 0.0 }
    }    
    
    pub fn sin(x: f64, _y: f64) -> f64 {
        x.sin()
    }
    
    pub fn cos(x: f64, _y: f64) -> f64 {
        x.cos()
    }
    
    pub fn tan(x: f64, _y: f64) -> f64 {
        x.tan()
    }
    
    pub fn csc(x: f64, _y: f64) -> f64 {
        1.0 / x.sin()
    }
    
    pub fn sec(x: f64, _y: f64) -> f64 {
        1.0 / x.cos()
    }
    
    pub fn cot(x: f64, _y: f64) -> f64 {
        1.0 / x.tan()
    }    
    
    pub fn asin(x: f64, _y: f64) -> f64 {
        x.asin()
    }
    
    pub fn acos(x: f64, _y: f64) -> f64 {
        x.acos()
    }
    
    pub fn atan(x: f64, _y: f64) -> f64 {
        x.atan()
    }    
    
    pub fn exp(x: f64, _y: f64) -> f64 {
        x.exp()
    }
    
    pub fn ln(x: f64, _y: f64) -> f64 {
        x.ln()
    }
    
    pub fn log(x: f64, _y: f64) -> f64 {
        x.log(10.0)
    }
    
    pub fn root(x: f64, _y: f64) -> f64 {
        x.sqrt()
    }
}

#[derive(Debug)]
pub struct Program {
    pub prog:   Vec<Instruction>,
    pub free:   Vec<Reg>,
    pub fac:    Reg,
    pub vars:   HashMap<String, Reg>,  
    pub diffs:  HashMap<String, Reg>,  
    pub regs:   HashSet<Reg>
}

impl Program {
    pub fn new(sys: &System) -> Program {
        let mut vars: HashMap<String, (Option<Reg>, Option<Reg>)> = HashMap::new();
        
        let ns = sys.states.len();
        // let np = sys.params.len();
        
        let mut vars: HashMap<String, Reg> = HashMap::new();
        let mut diffs: HashMap<String, Reg> = HashMap::new();
        
        vars.insert(sys.iv.name.clone(), Reg(1));
        
        for (i, v) in sys.params.iter().enumerate() {
            vars.insert(v.name.clone(), Reg(2+2*ns+i));
        }
        
        for (i, v) in sys.states.iter().enumerate() {
            vars.insert(v.name.clone(), Reg(2+i));
            diffs.insert(v.name.clone(), Reg(2+ns+i));
        }
        
        let mut regs: HashSet<Reg> = HashSet::new();
        regs.insert(Reg(0));
        regs.insert(Reg(1));
        let _: Vec<bool> = vars.values().map(|x| regs.insert(*x)).collect();
        let _: Vec<bool> = diffs.values().map(|x| regs.insert(*x)).collect();        
        
        /*
        for (i, v) in sys.obs.iter().enumerate() {
            vars.insert(v.lhs.var().unwrap(), Reg(2+2*ns+np+i));
        }
        */        
    
        Program {
            prog:   Vec::new(),
            free:   Vec::new(),
            fac:    sys.reg_base(),
            vars,
            diffs,
            regs
        }
    }
    
    pub fn push(&mut self, s: Instruction) {
        self.prog.push(s)
    }
    
    pub fn alloc_reg(&mut self) -> Reg {
        match self.free.pop() {
            Some(r) => r,
            None => {
                let r = self.fac;
                self.fac.advance();
                r
            }
        }
    }
    
    pub fn free_reg(&mut self, r: Reg) {
        if !self.regs.contains(&r) {
            self.free.push(r)
        }
    }
    
    pub fn reg(&self, name: &str) -> Reg {        
        *self.vars.get(name).expect(format!("cannot find {}", name).as_ref())
    }
    
    pub fn reg_diff(&self, name: &str) -> Reg {
        *self.diffs.get(name).expect(format!("cannot find diff_{}", name).as_ref())
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
        let dst = prog.alloc_reg();            
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
        prog.free_reg(x);
        dst
    }
    
    fn lower_binary(&self, prog: &mut Program, op: &str, args: &Vec<Expr>) -> Reg { 
        let x = args[0].lower(prog);
        let y = args[1].lower(prog);
        let dst = prog.alloc_reg();        
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
        prog.free_reg(x);
        prog.free_reg(y);
        dst
    }
    
    fn lower_ternary(&self, prog: &mut Program, op: &str, args: &Vec<Expr>) -> Reg {         
        if op != "ifelse" {
            return self.lower_poly(prog, op, args);
        }
        
        let x = args[0].lower(prog);
        let y1 = args[1].lower(prog);
        let y2 = args[2].lower(prog);
        let t1 = prog.alloc_reg();
        let t2 = prog.alloc_reg();    
        let dst = prog.alloc_reg();       
        prog.push(Instruction::Op { op: "if_pos".to_string(), f: Code::if_pos, x, y: y1, dst: t1 });
        prog.push(Instruction::Op { op: "if_neg".to_string(), f: Code::if_neg, x, y: y2, dst: t2 });
        prog.push(Instruction::Op { op: "plus".to_string(), f: Code::plus, x: t1, y: t2, dst });
        
        prog.free_reg(x);
        prog.free_reg(y1);
        prog.free_reg(y2);
        prog.free_reg(t1);
        prog.free_reg(t2);
        
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
            let dst = prog.alloc_reg();
            prog.push(Instruction::Op { op: op.to_string(), f, x, y, dst });
            prog.free_reg(x);
            x = dst;
        };
        
        x
    }
}

impl Lower for Expr {
    fn lower(&self, prog: &mut Program) -> Reg {
        match self {
            Expr::Const { val } => {
                let dst = prog.alloc_reg();
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
            if prog.vars.contains_key(&var) {
                panic!("duplicate assignment");
            }
            let r = prog.alloc_reg();
            prog.vars.insert(var, r);
            prog.regs.insert(r);
            r
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
