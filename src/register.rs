use std::collections::HashMap;

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Reg(pub usize);

impl Reg {
    pub fn advance(&mut self) {
        self.0 += 1        
    }    
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum RegType {
    Const,
    Var(String),
    State(String),
    Diff(String),
    Param(String),
    Obs(String),
    Temp
}


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
            freed: Vec::new()
        };
        
        f.alloc(RegType::Const, Some(0.0));
        f.alloc(RegType::Const, Some(-1.0));
        
        f
    }
    
    pub fn alloc(&mut self, t: RegType, val: Option<f64>) -> Reg {
        if let RegType::Temp = &t {
            if !self.freed.is_empty() {
                return self.freed.pop().unwrap()
            }    
        }
    
        let idx = self.regs.len();
        
        match &t {
            RegType::Const | 
            RegType::Temp => {},
            RegType::Var(_) |
            RegType::State(_) | 
            RegType::Diff(_) | 
            RegType::Param(_) | 
            RegType::Obs(_) => {
                self.lookup.insert(t.clone(), idx).map(|x| panic!("key already exists"));
            }
        };
        
        self.regs.push((t, val));       
        Reg(idx)
    }
    
    pub fn free(&mut self, r: Reg) {        
        if let RegType::Temp = self.regs[r.0].0 {
            self.freed.push(r);
        };
    }
    
    pub fn value(&self, r: Reg) -> Option<f64> {
        self.regs[r.0].1    
    }
    
    pub fn find(&self, t: &RegType) -> Option<Reg> {
        self.lookup.get(t).map(|idx| Reg(*idx))
    }
    
    pub fn count_states(&self) -> usize {
        self.regs.iter().filter(|x| matches!(x.0, RegType::State(_))).count()
    }
    
    pub fn count_params(&self) -> usize {
        self.regs.iter().filter(|x| matches!(x.0, RegType::Param(_))).count()
    }
    
    pub fn count_obs(&self) -> usize {
        self.regs.iter().filter(|x| matches!(x.0, RegType::Obs(_))).count()
    }
    
    pub fn count_temp(&self) -> usize {
        self.regs.iter().filter(|x| matches!(x.0, RegType::Temp)).count()
    }
    
    pub fn first_state(&self) -> Option<usize> {
        self.regs.iter().position(|x| matches!(x.0, RegType::State(_)))
    }
    
    pub fn first_param(&self) -> Option<usize> {
        self.regs.iter().position(|x| matches!(x.0, RegType::Param(_)))
    }   
    
    pub fn mem(&self) -> Vec<f64> {
        self.regs
            .iter()
            .map(|x| x.1.unwrap_or(0.0))
            .collect::<Vec<f64>>()            
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
