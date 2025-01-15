use anyhow::Result;
use std::fmt::Write;
use wasmtime::*;

use crate::code::*;
use crate::model::Program;
use crate::register::Word;
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
    x: Option<Word>,
    y: Option<Word>,
    z: Option<Word>,
    dst: Option<Word>,
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
            "mov" => OpType::Unary("f64.store", ArgType::F64, ArgType::F64),
            "neg" => OpType::Unary("f64.neg", ArgType::F64, ArgType::F64),
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
}

impl Compiler<WasmCode> for WasmCompiler {
    fn compile(&mut self, prog: &Program) -> WasmCode {
        self.push("(module");
        self.imports();
        self.push("(memory $memory 1)");
        self.push("(export \"memory\" (memory $memory))");

        self.push("(func $run");

        for c in prog.code.iter() {
            // self.push(format!(";; {}", c).as_str());
            match c {
                Instruction::Unary { op, .. } => {
                    if let OpType::Unary(s, ..) = self.op_code(op) {
                        self.push(s);
                    } else {
                        panic!("unkown unary op");
                    }
                }
                Instruction::Binary { op, .. } => {
                    if let OpType::Binary(s, ..) = self.op_code(op) {
                        self.push(s);
                    } else {
                        panic!("unkown binary op");
                    }
                }
                Instruction::IfElse { .. } => {
                    self.push("select");
                }
                Instruction::Eq { dst } => {
                    self.push(format!("i32.const {}", 8 * dst.0).as_str());
                }
                Instruction::Num { val, .. } => self.push(format!("f64.const {}", val).as_str()),
                Instruction::Var { reg, .. } => {
                    self.push(format!("(f64.load (i32.const {}))", 8 * reg.0).as_str())
                }
                _ => {}
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
        let module = Module::new(&engine, wat.as_str())?;
        let mut linker = Linker::<HostState>::new(&engine);

        Self::imports(&mut linker).expect("error in importing functions to wasm");

        let mut store: Store<HostState> = Store::new(&engine, 0);
        let instance = linker.instantiate(&mut store, &module)?;
        let run = instance.get_typed_func::<(), ()>(&mut store, "run")?;
        let memory = instance.get_memory(&mut store, "memory").unwrap();

        let p: &mut [f64] = unsafe { std::mem::transmute(memory.data_mut(&mut store)) };
        let _ = p[.._mem.len()].copy_from_slice(&_mem[..]);

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

    pub fn imports(linker: &mut Linker<HostState>) -> Result<()> {
        linker.func_wrap("code", "sin", |x: f64| -> f64 { x.sin() })?;
        linker.func_wrap("code", "cos", |x: f64| -> f64 { x.cos() })?;
        linker.func_wrap("code", "tan", |x: f64| -> f64 { x.tan() })?;
        linker.func_wrap("code", "csc", |x: f64| -> f64 { 1.0 / x.sin() })?;
        linker.func_wrap("code", "sec", |x: f64| -> f64 { 1.0 / x.cos() })?;
        linker.func_wrap("code", "cot", |x: f64| -> f64 { 1.0 / x.tan() })?;
        linker.func_wrap("code", "asin", |x: f64| -> f64 { x.asin() })?;
        linker.func_wrap("code", "acos", |x: f64| -> f64 { x.acos() })?;
        linker.func_wrap("code", "atan", |x: f64| -> f64 { x.atan() })?;
        linker.func_wrap("code", "exp", |x: f64| -> f64 { x.exp() })?;
        linker.func_wrap("code", "ln", |x: f64| -> f64 { x.ln() })?;
        linker.func_wrap("code", "log", |x: f64| -> f64 { x.log(10.0) })?;
        linker.func_wrap("code", "rem", |x: f64, y: f64| -> f64 { x % y })?;
        linker.func_wrap("code", "power", |x: f64, y: f64| -> f64 { x.powf(y) })?;

        Ok(())
    }
}

impl Compiled for WasmCode {
    fn run(&mut self) {
        self.run.call(&mut self.store, ()).unwrap();
    }

    #[inline]
    fn mem(&self) -> &[f64] {
        let p: &[f64] = unsafe { std::mem::transmute(self.memory.data(&self.store)) };
        p
    }

    #[inline]
    fn mem_mut(&mut self) -> &mut [f64] {
        let p: &mut [f64] = unsafe { std::mem::transmute(self.memory.data_mut(&mut self.store)) };
        p
    }
}
