/*
*   This file contains stubs for Vector.
*   The full Vector impl is in vector.rs and is used to 
*   generate the binary output but is not needed for lib.
*/

use std::ops::{Deref, DerefMut};
use crate::model::Program;

#[derive(Debug, Clone, PartialEq)]
pub struct Vector (pub Vec<f64>);

/**************** Deref *********************/

impl Deref for Vector {
    type Target = Vec<f64>;
    
    fn deref(&self) -> &Vec<f64> {
        &self.0
    }
}

impl DerefMut for Vector {
    fn deref_mut(&mut self) -> &mut Vec<f64> {
        &mut self.0
    }
}

/********************************************/

pub trait Callable {
    fn call(&mut self, du: &mut Vector, u: &Vector, t: f64);   
}

/********************************************/

pub trait Compiled {
    fn run(&self, mem: &mut [f64]);    
}

pub trait Compiler<T: Compiled> {
    fn compile(&mut self, prog: &Program) -> T;
}

