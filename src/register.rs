use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;

// Unit-like structure abstracting a single register
// it covers the index of the register in mem
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Reg(pub usize);

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize)]
// Adjacency tagging (https://serde.rs/enum-representations.html)
#[serde(tag = "t", content = "c")]
pub enum RegType {
    Const,
    Var(String),
    State(String),
    Diff(String),
    Param(String),
    Obs(String),
    Temp,
}

// The register file
#[derive(Debug)]
pub struct Frame {
    pub regs: Vec<(RegType, Option<f64>)>,
    pub lookup: HashMap<RegType, usize>,
    pub freed: Vec<Reg>,
}

impl Frame {
    pub fn new() -> Frame {
        let mut f = Frame {
            regs: Vec::new(),
            lookup: HashMap::new(),
            freed: Vec::new(),
        };

        f.alloc(RegType::Const, Some(0.0));
        f.alloc(RegType::Const, Some(1.0));
        f.alloc(RegType::Const, Some(-1.0));
        f.alloc(RegType::Const, Some(-0.0)); // MSB is 1, all other bits are 0, used for negation by xoring

        f
    }

    pub fn alloc(&mut self, t: RegType, val: Option<f64>) -> Reg {
        if let RegType::Temp = &t {
            if !self.freed.is_empty() {
                return self.freed.pop().unwrap();
            }
        }

        let idx = self.regs.len();

        match &t {
            RegType::Const | RegType::Temp => {}
            RegType::Var(_)
            | RegType::State(_)
            | RegType::Diff(_)
            | RegType::Param(_)
            | RegType::Obs(_) => {
                self.lookup
                    .insert(t.clone(), idx)
                    .map(|_x| panic!("key already exists"));
            }
        };

        self.regs.push((t, val));
        Reg(idx)
    }

    pub fn free(&mut self, r: Reg) {
        // only Temp tegisters can be recycled
        if let RegType::Temp = self.regs[r.0].0 {
            self.freed.push(r);
        };
    }

    pub fn value(&self, r: &Reg) -> Option<f64> {
        self.regs[r.0].1
    }

    pub fn is_diff(&self, r: &Reg) -> bool {
        if let (RegType::Diff(_), _) = self.regs[r.0] {
            true
        } else {
            false
        }
    }

    pub fn find(&self, t: &RegType) -> Option<Reg> {
        self.lookup.get(t).map(|idx| Reg(*idx))
    }

    pub fn count_states(&self) -> usize {
        self.regs
            .iter()
            .filter(|x| matches!(x.0, RegType::State(_)))
            .count()
    }

    pub fn count_params(&self) -> usize {
        self.regs
            .iter()
            .filter(|x| matches!(x.0, RegType::Param(_)))
            .count()
    }

    pub fn count_obs(&self) -> usize {
        self.regs
            .iter()
            .filter(|x| matches!(x.0, RegType::Obs(_)))
            .count()
    }

    pub fn count_temp(&self) -> usize {
        self.regs
            .iter()
            .filter(|x| matches!(x.0, RegType::Temp))
            .count()
    }

    pub fn first_state(&self) -> Option<usize> {
        self.regs
            .iter()
            .position(|x| matches!(x.0, RegType::State(_)))
    }

    pub fn first_param(&self) -> Option<usize> {
        self.regs
            .iter()
            .position(|x| matches!(x.0, RegType::Param(_)))
    }

    pub fn mem(&self) -> Vec<f64> {
        self.regs
            .iter()
            .map(|x| x.1.unwrap_or(0.0))
            .collect::<Vec<f64>>()
    }

    pub fn as_json(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(&self.regs)?)
    }
}

#[test]
fn test_frame() {
    let mut f = Frame::new();
    let r1 = f.alloc(RegType::State("x".to_string()), Some(2.0));
    let r2 = f.alloc(RegType::Param("a".to_string()), Some(-5.0));
    let q1 = RegType::State("x".to_string());
    let q2 = RegType::Param("a".to_string());
    assert_eq!(f.value(f.find(&q1).unwrap()).unwrap(), 2.0);
    assert_eq!(f.value(f.find(&q2).unwrap()).unwrap(), -5.0);
    println!("{:?}", &f);
    println!("ns = {}", f.count_states());
    println!("mem = {:?}", f.mem());
}
