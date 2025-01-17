use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;

// Unit-like structure abstracting a single register
// it covers the index of the register in mem
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct Word(pub usize, pub usize); // index, version

#[derive(Debug, Clone, PartialEq, Serialize)]
// Adjacency tagging (https://serde.rs/enum-representations.html)
#[serde(tag = "t", content = "c")]
pub enum WordType {
    Const(f64),
    Var(String),
    State(String, f64),
    Diff(String),
    Param(String, f64),
    Obs(String),
    Temp,
}

impl WordType {
    pub fn value(&self) -> Option<f64> {
        match self {
            WordType::State(_, val) => Some(*val),
            WordType::Param(_, val) => Some(*val),
            WordType::Const(val) => Some(*val),
            _ => None,
        }
    }
}

// The register file
#[derive(Debug)]
pub struct Frame {
    pub words: Vec<WordType>,
    pub named: HashMap<String, usize>,
    pub freed: Vec<Word>,
}

impl Frame {
    pub const ZERO: Word = Word(0, 0);
    pub const ONE: Word = Word(1, 0);
    pub const MINUS_ONE: Word = Word(2, 0);
    pub const MINUS_ZERO: Word = Word(3, 0);

    pub fn new() -> Frame {
        let mut f = Frame {
            words: Vec::new(),
            named: HashMap::new(),
            freed: Vec::new(),
        };

        f.alloc(WordType::Const(0.0));
        f.alloc(WordType::Const(1.0));
        f.alloc(WordType::Const(-1.0));
        f.alloc(WordType::Const(-0.0)); // MSB is 1, all other bits are 0, used for negation by xoring

        f
    }

    fn alloc_temp(&mut self) -> Word {
        if let Some(Word(idx, k)) = self.freed.pop() {
            Word(idx, k + 1) // because temps can share the same memory, version
                             // is increased to differentiate different temps
        } else {
            let idx = self.words.len();
            self.words.push(WordType::Temp);
            Word(idx, 0)
        }
    }

    pub fn alloc(&mut self, t: WordType) -> Word {
        let idx = self.words.len();

        match &t {
            WordType::Temp => {
                return self.alloc_temp();
            }
            WordType::Const(_) => {}
            WordType::Var(s) | WordType::State(s, _) | WordType::Param(s, _) | WordType::Obs(s) => {
                self.named
                    .insert(s.clone(), idx)
                    .map(|_x| panic!("key already exists"));
            }
            WordType::Diff(s) => {
                self.named
                    .insert(format!("δ{}", s), idx)
                    .map(|_x| panic!("diff key already exists"));
            }
        };

        self.words.push(t);
        Word(idx, 0)
    }

    pub fn free(&mut self, r: Word) {
        // only Temp tegisters can be recycled
        if let WordType::Temp = self.words[r.0] {
            self.freed.push(r);
        };
    }

    pub fn is_diff(&self, r: &Word) -> bool {
        if let WordType::Diff(_) = self.words[r.0] {
            true
        } else {
            false
        }
    }

    pub fn is_temp(&self, r: &Word) -> bool {
        if let WordType::Temp = self.words[r.0] {
            true
        } else {
            false
        }
    }

    pub fn is_obs(&self, r: &Word) -> bool {
        if let WordType::Obs(_) = self.words[r.0] {
            true
        } else {
            false
        }
    }

    pub fn find(&self, s: &str) -> Option<Word> {
        self.named.get(s).map(|idx| Word(*idx, 0))
    }

    pub fn find_diff(&self, s: &str) -> Option<Word> {
        let s = format!("δ{}", s);
        self.find(s.as_str())
    }

    pub fn count_states(&self) -> usize {
        self.words
            .iter()
            .filter(|x| matches!(x, WordType::State(_, _)))
            .count()
    }

    pub fn count_params(&self) -> usize {
        self.words
            .iter()
            .filter(|x| matches!(x, WordType::Param(_, _)))
            .count()
    }

    pub fn count_obs(&self) -> usize {
        self.words
            .iter()
            .filter(|x| matches!(x, WordType::Obs(_)))
            .count()
    }

    pub fn count_temp(&self) -> usize {
        self.words
            .iter()
            .filter(|x| matches!(x, WordType::Temp))
            .count()
    }

    pub fn first_state(&self) -> Option<usize> {
        self.words
            .iter()
            .position(|x| matches!(x, WordType::State(_, _)))
    }

    pub fn first_param(&self) -> Option<usize> {
        self.words
            .iter()
            .position(|x| matches!(x, WordType::Param(_, _)))
    }

    pub fn mem(&self) -> Vec<f64> {
        self.words
            .iter()
            .map(|x| x.value().unwrap_or(0.0))
            .collect::<Vec<f64>>()
    }

    pub fn as_json(&self) -> Result<String, Box<dyn Error>> {
        Ok(serde_json::to_string(&self.words)?)
    }
}
