use anyhow::Result;
use wasmi::*;
use wat;

use std::fmt::Write;

use crate::code::*;
use crate::model::Program;
use crate::register::Reg;
use crate::utils::*;

#[derive(Debug)]
enum OpType {
    None, // not implemented yet!
    Unary(&'static str),
    Binary(&'static str),
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
        for i in 0..4 * self.lev {
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
            "mov" => OpType::Unary("f64.add"),
            "plus" => OpType::Binary("f64.add"),
            "minus" => OpType::Binary("f64.sub"),
            "neg" => OpType::Unary("f64.neg"),
            "times" => OpType::Binary("f64.mul"),
            "divide" => OpType::Binary("f64.div"),
            "rem" => OpType::Binary("call $rem"),
            "power" => OpType::Binary("call $power"),
            "gt" => OpType::Binary("call $gt"),
            "geq" => OpType::Binary("call $geq"),
            "lt" => OpType::Binary("call $lt"),
            "leq" => OpType::Binary("call $leq"),
            "eq" => OpType::Binary("call $eq"),
            "neq" => OpType::Binary("call $neq"),
            "and" => OpType::Binary("call $and"),
            "or" => OpType::Binary("call $or"),
            "xor" => OpType::Binary("call $xor"),
            "if_pos" => OpType::Binary("call $if_pos"),
            "if_neg" => OpType::Binary("call $if_neg"),
            "sin" => OpType::Unary("call $sin"),
            "cos" => OpType::Unary("call $cos"),
            "tan" => OpType::Unary("call $tan"),
            "csc" => OpType::Unary("call $csc"),
            "sec" => OpType::Unary("call $sec"),
            "cot" => OpType::Unary("call $cot"),
            "arcsin" => OpType::Unary("call $asin"),
            "arccos" => OpType::Unary("call $acos"),
            "arctan" => OpType::Unary("call $atan"),
            "exp" => OpType::Unary("call $exp"),
            "ln" => OpType::Unary("call $ln"),
            "log" => OpType::Unary("call $log"),
            "root" => OpType::Unary("f64.sqrt"),
            _ => {
                let msg = format!("op_code {} not found", op);
                panic!("{}", msg);
            }
        }
    }

    fn imports(&mut self) {
        for s in [
            "sin", "cos", "tan", "csc", "sec", "cot", "asin", "acos", "atan", "exp", "ln", "log",
            "rem", "power", "gt", "geq", "lt", "leq", "eq", "neq", "or", "and", "xor", "if_pos",
            "if_neg", "root", "neg",
        ] {
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
            match c {
                Instruction::Num { .. } => {} // Num and Var do not generate any code
                Instruction::Var { .. } => {} // They are mainly for debugging
                Instruction::Op { x, y, dst, op, .. } => {
                    let s = match self.op_code(op) {
                        OpType::Unary(s) => {
                            format!("(f64.store (i32.const {}) ({} (f64.load (i32.const {})) (f64.const 0)))", 
                                    8*dst.0, s, 8*x.0)
                        }
                        OpType::Binary(s) => {                            
                            format!("(f64.store (i32.const {}) ({} (f64.load (i32.const {})) (f64.load (i32.const {}))))", 
                                    8*dst.0, s, 8*x.0, 8*y.0)
                        }
                        OpType::None => {
                            panic!("unknown op code")
                        }
                    };
                    self.push(s.as_str());
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

        Self::imports(&mut store, &mut linker);

        let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;

        let run = instance.get_typed_func::<(), ()>(&store, "run")?;
        let memory = instance.get_memory(&store, "memory").unwrap();

        let p: &mut [f64] = unsafe {
            std::slice::from_raw_parts_mut(memory.data_ptr(&store) as *mut f64, _mem.len())
        };

        p.copy_from_slice(&_mem[..]);

        let mut wasm = WasmCode {
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
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { x.sin() }),
            ),
        )?;

        linker.define(
            "code",
            "cos",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { x.cos() }),
            ),
        )?;

        linker.define(
            "code",
            "tan",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { x.tan() }),
            ),
        )?;

        linker.define(
            "code",
            "csc",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { 1.0 / x.sin() }),
            ),
        )?;

        linker.define(
            "code",
            "sec",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { 1.0 / x.cos() }),
            ),
        )?;

        linker.define(
            "code",
            "cot",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { 1.0 / x.tan() }),
            ),
        )?;

        linker.define(
            "code",
            "asin",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { x.asin() }),
            ),
        )?;

        linker.define(
            "code",
            "acos",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { x.acos() }),
            ),
        )?;

        linker.define(
            "code",
            "atan",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { x.atan() }),
            ),
        )?;

        linker.define(
            "code",
            "exp",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { x.exp() }),
            ),
        )?;

        linker.define(
            "code",
            "ln",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { x.ln() }),
            ),
        )?;

        linker.define(
            "code",
            "log",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { x.log(10.0) }),
            ),
        )?;

        linker.define(
            "code",
            "rem",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 { x % y }),
            ),
        )?;

        linker.define(
            "code",
            "power",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 { x.powf(y) }),
            ),
        )?;

        linker.define(
            "code",
            "gt",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 {
                    if x > y {
                        1.0
                    } else {
                        -1.0
                    }
                }),
            ),
        )?;

        linker.define(
            "code",
            "geq",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 {
                    if x >= y {
                        1.0
                    } else {
                        -1.0
                    }
                }),
            ),
        )?;

        linker.define(
            "code",
            "lt",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 {
                    if x < y {
                        1.0
                    } else {
                        -1.0
                    }
                }),
            ),
        )?;

        linker.define(
            "code",
            "leq",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 {
                    if x <= y {
                        1.0
                    } else {
                        -1.0
                    }
                }),
            ),
        )?;

        linker.define(
            "code",
            "eq",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 {
                    if x == y {
                        1.0
                    } else {
                        -1.0
                    }
                }),
            ),
        )?;

        linker.define(
            "code",
            "neq",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 {
                    if x != y {
                        1.0
                    } else {
                        -1.0
                    }
                }),
            ),
        )?;

        linker.define(
            "code",
            "and",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 {
                    if x > 0.0 && y > 0.0 {
                        1.0
                    } else {
                        -1.0
                    }
                }),
            ),
        )?;

        linker.define(
            "code",
            "or",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 {
                    if x > 0.0 || y > 0.0 {
                        1.0
                    } else {
                        -1.0
                    }
                }),
            ),
        )?;

        linker.define(
            "code",
            "xor",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 {
                    if x * y < 0.0 {
                        1.0
                    } else {
                        -1.0
                    }
                }),
            ),
        )?;

        linker.define(
            "code",
            "if_pos",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 {                    
                    if x > 0.0 {
                        y
                    } else {
                        0.0
                    }
                }),
            ),
        )?;

        linker.define(
            "code",
            "if_neg",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, y: f64| -> f64 {
                    if x < 0.0 {
                        y
                    } else {
                        0.0
                    }
                }),
            ),
        )?;

        linker.define(
            "code",
            "root",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { x.sqrt() }),
            ),
        )?;

        linker.define(
            "code",
            "neg",
            Func::wrap(
                &mut *store,
                (|caller: Caller<'_, HostState>, x: f64, _y: f64| -> f64 { -x }),
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
