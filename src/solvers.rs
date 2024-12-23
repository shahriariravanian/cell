use std::ops::Range;
use std::fmt;

use crate::vector::*;

pub trait Callable {
    fn call(&mut self, du: &mut Vector, u: &Vector, t: f64);   
}

#[derive(Debug, Clone)]
pub struct Row {pub t: f64, pub x: Vector}

impl fmt::Display for Row {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.t);
        let x = &self.x;
        for j in 0..x.len() {
            write!(f, "\t{}", x[j]);
        }   
        writeln!(f, "");
        Ok(())
    }    
}

pub trait Solver<F> where F: Callable {
    fn solve(&self, f: &mut F, u0: &Vector, ts: Range<f64>) -> Vec<Row>;
}

pub struct Euler {
    dt: f64,
    stride: usize
}

impl Euler {
    pub fn new(dt: f64, stride: usize) -> Euler {
        Euler { dt, stride }
    }
}

impl<F: Callable> Solver<F> for Euler {
    fn solve(&self, f: &mut F, u0: &Vector, ts: Range<f64>) -> Vec<Row> {
        let mut u = u0.clone();
        let mut du = u.clone();
        
        let n = ((ts.end - ts.start) / self.dt).floor() as usize;
        let mut sol = Vec::new();
        
        for i in 0..n {
            let t = i as f64 * self.dt;
        
            if i % self.stride == 0 {
                sol.push(Row { t, x: u.clone() });
            }            

            f.call(&mut du, &u, t);        
        
            u += &du * self.dt;
        };        
        
        sol
    }
}
