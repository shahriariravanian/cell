#[macro_use]
mod macros;

use super::analyzer::Analyzer;
use super::code::*;
use super::machine::MachineCode;
use super::model::Program;
use super::register::{Frame, Word};
use super::utils::*;

#[derive(Debug)]
pub struct ArmCompiler {
    machine_code: Vec<u8>,
    x4: Option<Word>,
    x5: Option<Word>,
}

impl ArmCompiler {
    const D0: u8 = 0;
    const D1: u8 = 1;
    const D2: u8 = 2;
    const D3: u8 = 3;
    const D4: u8 = 4;
    const D5: u8 = 5;
    const D6: u8 = 6;
    const D7: u8 = 7;

    pub fn new() -> ArmCompiler {
        Self {
            machine_code: Vec::new(),
            x4: None,
            x5: None,
        }
    }

    pub fn push_u32(&mut self, w: u32) {
        self.machine_code.push(w as u8);
        self.machine_code.push((w >> 8) as u8);
        self.machine_code.push((w >> 16) as u8);
        self.machine_code.push((w >> 24) as u8);
    }

    fn op_code(&mut self, op: &str, p: Proc) {
        match op {
            "mov" => {}
            "plus" => self.push_u32(arm! {fadd d(0), d(0), d(1)}),
            "minus" => self.push_u32(arm! {fsub d(0), d(0), d(1)}),
            "times" => self.push_u32(arm! {fmul d(0), d(0), d(1)}),
            "divide" => self.push_u32(arm! {fdiv d(0), d(0), d(1)}),
            "gt" => self.push_u32(arm! {fcmgt d(0), d(0), d(1)}),
            "geq" => self.push_u32(arm! {fcmge d(0), d(0), d(1)}),
            "lt" => self.push_u32(arm! {fcmlt d(0), d(0), d(1)}),
            "leq" => self.push_u32(arm! {fcmle d(0), d(0), d(1)}),
            "eq" => self.push_u32(arm! {fcmeq d(0), d(0), d(1)}),
            "and" => self.push_u32(arm! {and v(0).8b, v(0).8b, v(1).8b}),
            "or" => self.push_u32(arm! {orr v(0).8b, v(0).8b, v(1).8b}),
            "xor" => self.push_u32(arm! {eor v(0).8b, v(0).8b, v(1).8b}),
            "neg" => self.push_u32(arm! {fneg d(0), d(0)}),
            "root" => self.push_u32(arm! {fsqrt d(0), d(0)}),
            "neq" => {
                self.push_u32(arm! {fcmeq d(0), d(0), d(1)});
                self.push_u32(arm! {not v(0).8b, v(0).8b});
            }
            _ => {
                // self.dump_buffer();
                self.push_u32(arm! {ldr x(0), [x(20), #8*p.0]});
                self.push_u32(arm! {blr x(0)});
            }
        }
    }

    // d2 == true ? d0 : d1
    fn ifelse(&mut self) {
        self.push_u32(arm! {and v(0).8b, v(0).8b, v(2).8b});
        self.push_u32(arm! {not v(2).8b, v(2).8b});
        self.push_u32(arm! {and v(1).8b, v(1).8b, v(2).8b});
        self.push_u32(arm! {orr v(0).8b, v(0).8b, v(1).8b});
    }

    fn load_xmm_indirect(&mut self, x: u8, r: Word) {
        if r == Frame::ZERO {
            self.push_u32(arm! {fmov d(x), #0.0});
        } else if r == Frame::ONE {
            self.push_u32(arm! {fmov d(x), #1.0});
        } else if r == Frame::MINUS_ONE {
            self.push_u32(arm! {fmov d(x), #-1.0});
        } else {
            self.push_u32(arm! {ldr d(x), [x(19), #8*r.0]});
        }
    }

    fn save_xmm_indirect(&mut self, x: u8, r: Word) {
        if r.0 > 2 {
            self.push_u32(arm! {str d(x), [x(19), #8*r.0]});
        }
    }

    fn load_buffered(&mut self, x: u8, r: Word) {
        if self.x4.is_some_and(|s| s == r) {
            self.push_u32(arm! {fmov d(x), d(4)});
            self.x4 = None;
            return;
        }

        if self.x5.is_some_and(|s| s == r) {
            self.push_u32(arm! {fmov d(x), d(5)});
            self.x5 = None;
            return;
        }

        self.load_xmm_indirect(x, r);
    }

    fn save_buffered(&mut self, x: u8, r: Word) {
        if self.x4.is_none() {
            self.push_u32(arm! {fmov d(4), d(x)});
            self.x4 = Some(r);
            return;
        }

        if self.x5.is_none() {
            self.push_u32(arm! {fmov d(5), d(x)});
            self.x5 = Some(r);
            return;
        }

        self.save_xmm_indirect(x, r);
    }

    fn dump_buffer(&mut self) {
        if let Some(s) = self.x4 {
            self.save_xmm_indirect(Self::D4, s);
            self.x4 = None;
        }

        if let Some(s) = self.x5 {
            self.save_xmm_indirect(Self::D5, s);
            self.x5 = None;
        }
    }

    fn prologue(&mut self) {
        self.push_u32(arm! {sub sp, sp, #32});
        self.push_u32(arm! {str lr, [sp, #0]});
        self.push_u32(arm! {stp x(19), x(20), [sp, #16]});
        self.push_u32(arm! {mov x(19), x(0)});
        self.push_u32(arm! {mov x(20), x(2)});
    }

    fn epilogue(&mut self) {
        self.push_u32(arm! {ldp x(19), x(20), [sp, #16]});
        self.push_u32(arm! {ldr lr, [sp, #0]});
        self.push_u32(arm! {add sp, sp, #32});
        self.push_u32(arm! {ret});
    }
}

impl Compiler<MachineCode> for ArmCompiler {
    fn compile(&mut self, prog: &Program) -> MachineCode {
        self.prologue();

        let mut r = Frame::ZERO;

        let analyzer = Analyzer::new(prog);
        let saveable = analyzer.find_saveable();
        let bufferable = analyzer.find_bufferable();

        for c in prog.code.iter() {
            match c {
                Instruction::Unary { p, x, dst, op } => {
                    if r != *x {
                        self.load_buffered(Self::D0, *x);
                    };
                    self.op_code(&op, *p);
                    r = *dst;
                }
                Instruction::Binary { p, x, y, dst, op } => {
                    // commutative operators
                    let (x, y) = if (op == "plus" || op == "times") && *y == r {
                        (y, x)
                    } else {
                        (x, y)
                    };

                    if *y == r {
                        self.push_u32(arm! {fmov d(1), d(0)});
                    } else {
                        self.load_buffered(Self::D1, *y);
                    }

                    if *x != r {
                        self.load_buffered(Self::D0, *x);
                    }

                    self.op_code(&op, *p);
                    r = *dst;
                }
                Instruction::IfElse { x1, x2, cond, dst } => {
                    if *cond == r {
                        self.push_u32(arm! {fmov d(2), d(0)});
                    } else {
                        self.load_buffered(Self::D2, *cond);
                    }

                    if *x2 == r {
                        self.push_u32(arm! {fmov d(1), d(0)});
                    } else {
                        self.load_buffered(Self::D1, *x2);
                    }

                    if *x1 != r {
                        self.load_buffered(Self::D0, *x1);
                    }

                    self.ifelse();
                    r = *dst;
                }
                _ => {
                    continue;
                }
            }

            // A diff register should be saved, cannot be buffered
            if prog.frame.is_diff(&r) {
                self.save_xmm_indirect(Self::D0, r);
                r = Frame::ZERO;
            }

            // A bufferable register can be buffered without the
            // need for self.dump_buffer()
            if bufferable.contains(&r) {
                self.save_buffered(Self::D0, r);
                r = Frame::ZERO;
            }

            // A saveable register can be saved directly or buffered
            // However, if it is buffered, self.dump_buffer() should be
            // uncommented in fn op_code
            if saveable.contains(&r) {
                self.save_xmm_indirect(Self::D0, r);
                r = Frame::ZERO;
            }
        }

        self.epilogue();

        MachineCode::new(
            &self.machine_code.clone(),
            prog.virtual_table(),
            prog.frame.mem(),
        )
    }
}
