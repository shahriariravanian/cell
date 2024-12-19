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
