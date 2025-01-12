use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;

// Unit-like structure abstracting a single register
// it covers the index of the register in mem
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Reg(pub usize);

#[derive(Debug, Clone, PartialEq, Serialize)]
// Adjacency tagging (https://serde.rs/enum-representations.html)
#[serde(tag = "t", content = "c")]
pub enum RegType {
    Const(f64),
    Var(String),
    State(String, f64),
    Diff(String),
    Param(String, f64),
    Obs(String),
    Temp,
}

impl RegType {
    pub fn value(&self) -> Option<f64> {
        match self {
            RegType::State(_, val) => Some(*val),
            RegType::Param(_, val) => Some(*val),
            RegType::Const(val) => Some(*val),
            _ => None,
        }
    }
}

// The register file
#[derive(Debug)]
pub struct Frame {
    pub regs: Vec<RegType>,
    pub named: HashMap<String, usize>,
    pub freed: Vec<Reg>,
}

impl Frame {
    pub fn new() -> Frame {
        let mut f = Frame {
            regs: Vec::new(),
            named: HashMap::new(),
            freed: Vec::new(),
        };

        f.alloc(RegType::Const(0.0));
        f.alloc(RegType::Const(1.0));
        f.alloc(RegType::Const(-1.0));
        f.alloc(RegType::Const(-0.0)); // MSB is 1, all other bits are 0, used for negation by xoring

        f
    }

    fn alloc_temp(&mut self) -> Reg {
        if !self.freed.is_empty() {
            let k = {
                let m = self.freed.iter().min_by_key(|x| x.0).unwrap();
                self.freed.iter().position(|x| x == m).unwrap()
            };
            return self.freed.remove(k);
        };

        let idx = self.regs.len();
        self.regs.push(RegType::Temp);
        Reg(idx)
    }

    pub fn alloc(&mut self, t: RegType) -> Reg {
        let idx = self.regs.len();

        match &t {
            RegType::Temp => {
                return self.alloc_temp();
            }
            RegType::Const(_) => {}
            RegType::Var(s) | RegType::State(s, _) | RegType::Param(s, _) | RegType::Obs(s) => {
                self.named
                    .insert(s.clone(), idx)
                    .map(|_x| panic!("key already exists"));
            }
            RegType::Diff(s) => {
                self.named
                    .insert(format!("δ{}", s), idx)
                    .map(|_x| panic!("diff key already exists"));
            }
        };

        self.regs.push(t);
        Reg(idx)
    }

    pub fn free(&mut self, r: Reg) {
        // only Temp tegisters can be recycled
        if let RegType::Temp = self.regs[r.0] {
            self.freed.push(r);
        };
    }

    pub fn is_diff(&self, r: &Reg) -> bool {
        if let RegType::Diff(_) = self.regs[r.0] {
            true
        } else {
            false
        }
    }

    pub fn is_temp(&self, r: &Reg) -> bool {
        if let RegType::Temp = self.regs[r.0] {
            true
        } else {
            false
        }
    }

    pub fn find(&self, s: &str) -> Option<Reg> {
        self.named.get(s).map(|idx| Reg(*idx))
    }

    pub fn find_diff(&self, s: &str) -> Option<Reg> {
        let s = format!("δ{}", s);
        self.find(s.as_str())
    }

    pub fn count_states(&self) -> usize {
        self.regs
            .iter()
            .filter(|x| matches!(x, RegType::State(_, _)))
            .count()
    }

    pub fn count_params(&self) -> usize {
        self.regs
            .iter()
            .filter(|x| matches!(x, RegType::Param(_, _)))
            .count()
    }

    pub fn count_obs(&self) -> usize {
        self.regs
            .iter()
            .filter(|x| matches!(x, RegType::Obs(_)))
            .count()
    }

    pub fn count_temp(&self) -> usize {
        self.regs
            .iter()
            .filter(|x| matches!(x, RegType::Temp))
            .count()
    }

    pub fn first_state(&self) -> Option<usize> {
        self.regs
            .iter()
            .position(|x| matches!(x, RegType::State(_, _)))
    }

    pub fn first_param(&self) -> Option<usize> {
        self.regs
            .iter()
            .position(|x| matches!(x, RegType::Param(_, _)))
    }

    pub fn mem(&self) -> Vec<f64> {
        self.regs
            .iter()
            .map(|x| x.value().unwrap_or(0.0))
            .collect::<Vec<f64>>()
    }

    pub fn as_json(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(&self.regs)?)
    }
}
