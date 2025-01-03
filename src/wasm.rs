use anyhow::Result;
use wasmi::*;
use wat;

use std::fmt::Write;

use crate::code::*;
use crate::model::Program;
use crate::register::{Reg, RegType};
use crate::utils::*;

#[derive(Debug, Copy, Clone)]
enum ArgType {
    F64,
    I32,
}

#[derive(Debug)]
enum OpType {
    Nop(ArgType), // not implemented yet!
    Unary(&'static str, ArgType, ArgType),
    Binary(&'static str, ArgType, ArgType, ArgType),
    Ternary(&'static str, ArgType, ArgType, ArgType, ArgType),
}

#[derive(Debug, Clone)]
struct Op {
    op: String,
    x: Option<Reg>,
    y: Option<Reg>,
    z: Option<Reg>,
    dst: Option<Reg>,
    store: bool,
}

#[derive(Debug)]
pub struct WasmCompiler {
    buf: String,
    lev: i32,
}

impl WasmCompiler {
    pub fn new() -> WasmCompiler {
        Self {
            buf: String::new(),
            lev: 0,
        }
    }

    fn push(&mut self, s: &str) {
        for _ in 0..4 * self.lev {
            let _ = write!(self.buf, " ");
        }
        let _ = writeln!(self.buf, "{}", s);
        self.lev += s.chars().filter(|x| *x == '(').count() as i32;
        self.lev -= s.chars().filter(|x| *x == ')').count() as i32;
        if self.lev < 0 {
            panic!("more ) than (");
        }
    }

    fn op_code(&mut self, op: &str) -> OpType {
        match op {
            "mov" => OpType::Nop(ArgType::F64),
            "neg" => OpType::Unary("f64.negate", ArgType::F64, ArgType::F64),
            "sin" => OpType::Unary("call $sin", ArgType::F64, ArgType::F64),
            "cos" => OpType::Unary("call $cos", ArgType::F64, ArgType::F64),
            "tan" => OpType::Unary("call $tan", ArgType::F64, ArgType::F64),
            "csc" => OpType::Unary("call $csc", ArgType::F64, ArgType::F64),
            "sec" => OpType::Unary("call $sec", ArgType::F64, ArgType::F64),
            "cot" => OpType::Unary("call $cot", ArgType::F64, ArgType::F64),
            "arcsin" => OpType::Unary("call $asin", ArgType::F64, ArgType::F64),
            "arccos" => OpType::Unary("call $acos", ArgType::F64, ArgType::F64),
            "arctan" => OpType::Unary("call $atan", ArgType::F64, ArgType::F64),
            "exp" => OpType::Unary("call $exp", ArgType::F64, ArgType::F64),
            "ln" => OpType::Unary("call $ln", ArgType::F64, ArgType::F64),
            "log" => OpType::Unary("call $log", ArgType::F64, ArgType::F64),
            "root" => OpType::Unary("f64.sqrt", ArgType::F64, ArgType::F64),

            "plus" => OpType::Binary("f64.add", ArgType::F64, ArgType::F64, ArgType::F64),
            "minus" => OpType::Binary("f64.sub", ArgType::F64, ArgType::F64, ArgType::F64),
            "times" => OpType::Binary("f64.mul", ArgType::F64, ArgType::F64, ArgType::F64),
            "divide" => OpType::Binary("f64.div", ArgType::F64, ArgType::F64, ArgType::F64),
            "rem" => OpType::Binary("call $rem", ArgType::F64, ArgType::F64, ArgType::F64),
            "power" => OpType::Binary("call $power", ArgType::F64, ArgType::F64, ArgType::F64),
            "gt" => OpType::Binary("f64.gt", ArgType::I32, ArgType::F64, ArgType::F64),
            "geq" => OpType::Binary("f64.ge", ArgType::I32, ArgType::F64, ArgType::F64),
            "lt" => OpType::Binary("f64.lt", ArgType::I32, ArgType::F64, ArgType::F64),
            "leq" => OpType::Binary("f64.le", ArgType::I32, ArgType::F64, ArgType::F64),
            "eq" => OpType::Binary("f64.eq", ArgType::I32, ArgType::F64, ArgType::F64),
            "neq" => OpType::Binary("f64.ne", ArgType::I32, ArgType::F64, ArgType::F64),
            "and" => OpType::Binary("i32.and", ArgType::I32, ArgType::I32, ArgType::I32),
            "or" => OpType::Binary("i32.or", ArgType::I32, ArgType::I32, ArgType::I32),
            "xor" => OpType::Binary("i32.xor", ArgType::I32, ArgType::I32, ArgType::I32),
            //"if_pos" => OpType::Binary("call $if_pos", ArgType::F64, ArgType::I32, ArgType::F64),
            //"if_neg" => OpType::Binary("call $if_neg", ArgType::F64, ArgType::I32, ArgType::F64),
            "select" => OpType::Ternary(
                "select",
                ArgType::F64,
                ArgType::F64,
                ArgType::F64,
                ArgType::I32,
            ),
            _ => {
                let msg = format!("op_code {} not found", op);
                panic!("{}", msg);
            }
        }
    }

    fn imports(&mut self) {
        // unary
        for s in [
            "sin", "cos", "tan", "csc", "sec", "cot", "asin", "acos", "atan", "exp", "ln", "log",
        ] {
            let cmd = format!(
                "(import \"code\" \"{}\" (func ${} (param f64)(result f64)))",
                s, s
            );
            self.push(cmd.as_str());
        }

        // binary
        for s in ["rem", "power"] {
            let cmd = format!(
                "(import \"code\" \"{}\" (func ${} (param f64)(param f64)(result f64)))",
                s, s
            );
            self.push(cmd.as_str());
        }
    }

    fn reg_load(&mut self, prog: &Program, r: Option<Reg>, ty: ArgType) {
        if let Some(r) = r {
            let (t, val) = prog.frame.regs[r.0].clone();

            let p = match ty {
                ArgType::F64 => "f64",
                ArgType::I32 => "i32",
            };

            let s = match t {
                RegType::Const => {
                    format!("{}.const {}", p, val.unwrap_or(0.0))
                }
                _ => {
                    format!("({}.load (i32.const {}))", p, 8 * r.0)
                }
            };

            self.push(s.as_str());
        }
    }

    fn filter_code(&self, prog: &Program) -> Vec<Op> {
        let mut code: Vec<Op> = Vec::new();

        for c in prog.code.iter() {
            if let Instruction::Op { x, y, dst, op, .. } = c {
                if op == "if_pos" || op == "if_neg" {
                    code.push(Op {
                        x: if op == "if_pos" {
                            Some(*y)
                        } else {
                            Some(Reg(0))
                        },
                        y: if op == "if_neg" {
                            Some(*y)
                        } else {
                            Some(Reg(0))
                        },
                        z: Some(*x),
                        dst: Some(*dst),
                        op: (if *y == Reg(0) { "mov" } else { "select" }).to_string(),
                        store: true,
                    });
                } else {
                    code.push(Op {
                        x: Some(*x),
                        y: Some(*y),
                        z: None,
                        dst: Some(*dst),
                        op: op.clone(),
                        store: true,
                    });
                }
            }
        }

        for i in 0..code.len() - 1 {
            if code[i].dst == code[i + 1].x && code[i].x.is_some() {
                code[i].dst = code[i + 1].dst;
                code[i].store = false;
                code[i + 1].x = None;
                code[i + 1].dst = None;
            }
        }

        code
    }
}

impl Compiler<WasmCode> for WasmCompiler {
    fn compile(&mut self, prog: &Program) -> WasmCode {
        self.push("(module");
        self.imports();
        self.push("(memory $memory 1)");
        self.push("(export \"memory\" (memory $memory))");

        self.push("(func $run");

        let code = self.filter_code(prog);

        for c in code.iter() {
            let Op {
                x,
                y,
                z,
                dst,
                op,
                store,
            } = c;

            if let Some(dst) = dst {
                self.push(format!("i32.const {}", 8 * dst.0).as_str());
            }

            let dst_t = match self.op_code(op) {
                OpType::Unary(s, dst_t, x_t) => {
                    self.reg_load(prog, *x, x_t);
                    self.push(s);
                    dst_t
                }
                OpType::Binary(s, dst_t, x_t, y_t) => {
                    self.reg_load(prog, *x, x_t);
                    self.reg_load(prog, *y, y_t);
                    self.push(s);
                    dst_t
                }
                OpType::Ternary(s, dst_t, x_t, y_t, z_t) => {
                    self.reg_load(prog, *x, x_t);
                    self.reg_load(prog, *y, y_t);
                    self.reg_load(prog, *z, z_t);
                    self.push(s);
                    dst_t
                }
                OpType::Nop(dst_t) => {
                    self.reg_load(prog, *x, dst_t);
                    dst_t
                }
            };

            if *store {
                match dst_t {
                    ArgType::F64 => self.push("f64.store"),
                    ArgType::I32 => self.push("i32.store"),
                }
            }
        }

        self.push(")"); // (func $run
        self.push("(export \"run\" (func $run))");
        self.push(")"); // (module

        // println!("{}", self.buf);

        WasmCode::new(self.buf.clone(), prog.frame.mem()).unwrap()
    }
}

type HostState = u32;

#[derive(Debug)]
pub struct WasmCode {
    _mem: Vec<f64>,
    wat: String,
    engine: Engine,
    module: Module,
    store: Store<HostState>,
    linker: Linker<HostState>,
    instance: Instance,
    run: TypedFunc<(), ()>,
    memory: Memory,
}

impl WasmCode {
    fn new(wat: String, _mem: Vec<f64>) -> Result<WasmCode> {
        let engine = Engine::default();
        let wasm = wat::parse_str(wat.as_str())?;
        let module = Module::new(&engine, &mut &wasm[..])?;
        let mut store = Store::new(&engine, 0);
        let mut linker = <Linker<HostState>>::new(&engine);

        Self::imports(&mut store, &mut linker).expect("error in importing functions to wasm");

        let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;

        let run = instance.get_typed_func::<(), ()>(&store, "run")?;
        let memory = instance.get_memory(&store, "memory").unwrap();

        let p: &mut [f64] = unsafe {
            std::slice::from_raw_parts_mut(memory.data_ptr(&store) as *mut f64, _mem.len())
        };

        p.copy_from_slice(&_mem[..]);

        let wasm = WasmCode {
            _mem,
            wat,
            engine,
            module,
            store,
            linker,
            instance,
            run,
            memory,
        };

        Ok(wasm)
    }

    pub fn imports(store: &mut Store<HostState>, linker: &mut Linker<HostState>) -> Result<()> {
        linker.define(
            "code",
            "sin",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { x.sin() },
            ),
        )?;

        linker.define(
            "code",
            "cos",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { x.cos() },
            ),
        )?;

        linker.define(
            "code",
            "tan",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { x.tan() },
            ),
        )?;

        linker.define(
            "code",
            "csc",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { 1.0 / x.sin() },
            ),
        )?;

        linker.define(
            "code",
            "sec",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { 1.0 / x.cos() },
            ),
        )?;

        linker.define(
            "code",
            "cot",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { 1.0 / x.tan() },
            ),
        )?;

        linker.define(
            "code",
            "asin",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { x.asin() },
            ),
        )?;

        linker.define(
            "code",
            "acos",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { x.acos() },
            ),
        )?;

        linker.define(
            "code",
            "atan",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { x.atan() },
            ),
        )?;

        linker.define(
            "code",
            "exp",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { x.exp() },
            ),
        )?;

        linker.define(
            "code",
            "ln",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { x.ln() },
            ),
        )?;

        linker.define(
            "code",
            "log",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64| -> f64 { x.log(10.0) },
            ),
        )?;

        linker.define(
            "code",
            "rem",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 { x % y },
            ),
        )?;

        linker.define(
            "code",
            "power",
            Func::wrap(
                &mut *store,
                |_caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 { x.powf(y) },
            ),
        )?;

        Ok(())
    }
}

impl Compiled for WasmCode {
    fn run(&mut self) {
        self.run.call(&mut self.store, ()).unwrap();
    }

    #[inline]
    fn mem(&self) -> &[f64] {
        let p: &[f64] = unsafe {
            std::slice::from_raw_parts(
                self.memory.data_ptr(&self.store) as *const f64,
                self._mem.len(),
            )
        };
        p
    }

    #[inline]
    fn mem_mut(&mut self) -> &mut [f64] {
        let p: &mut [f64] = unsafe {
            std::slice::from_raw_parts_mut(
                self.memory.data_ptr(&self.store) as *mut f64,
                self._mem.len(),
            )
        };
        p
    }
}
