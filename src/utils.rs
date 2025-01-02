/*
*   This file contains stubs for Vector.
*   The full Vector impl is in vector.rs and is used to
*   generate the binary output but is not needed for lib.
*/

use crate::model::Program;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq)]
pub struct Vector(pub Vec<f64>);

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

impl Vector {
    pub fn as_ref(&self) -> &[f64] {
        self.0.as_ref()
    }

    pub fn as_mut(&mut self) -> &mut [f64] {
        self.0.as_mut()
    }
}

/********************************************/

pub trait Callable {
    fn call(&mut self, du: &mut [f64], u: &[f64], p: &[f64], t: f64);
}

/********************************************/

pub trait Compiled {
    fn run(&mut self);
    fn mem(&self) -> &[f64];
    fn mem_mut(&mut self) -> &mut [f64];
}

pub trait Compiler<T: Compiled> {
    fn compile(&mut self, prog: &Program) -> T;
}
