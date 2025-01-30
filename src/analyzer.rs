use std::collections::{HashSet, HashMap};

use super::code::Instruction;
use super::model::Program;
use super::register::Word;

pub enum Event {
    Producer(Word),
    Consumer(Word),
    Caller(String),
}

pub struct Analyzer {
    pub events: Vec<Event>,
}

impl Analyzer {
    pub fn new(prog: &Program) -> Self {
        let mut events: Vec<Event> = Vec::new();

        for c in prog.code.iter() {
            match c {
                Instruction::Unary { op, x, dst, .. } => {
                    events.push(Event::Consumer(*x));
                    events.push(Event::Caller(op.clone()));
                    events.push(Event::Producer(*dst));                    
                }
                Instruction::Binary { op, x, y, dst, .. } => {
                    events.push(Event::Consumer(*x));
                    events.push(Event::Consumer(*y));
                    events.push(Event::Caller(op.clone()));
                    events.push(Event::Producer(*dst));
                }
                Instruction::IfElse { x1, x2, cond, dst } => {
                    events.push(Event::Consumer(*x1));
                    events.push(Event::Consumer(*x2));
                    events.push(Event::Consumer(*cond));
                    events.push(Event::Caller("select".to_string()));
                    events.push(Event::Producer(*dst));
                }
                _ => {}
            }
        }

        Self { events }
    }

    /*
        A saveable register is produced but is not consumed immediately
        In other words, it cannot be coalesced over consecuative instructions
    */
    pub fn find_saveable(&self) -> HashSet<Word> {
        let mut candidates: Vec<Word> = Vec::new();
        let mut saveable: HashSet<Word> = HashSet::new();

        for l in self.events.iter() {
            match l {
                Event::Producer(p) => {
                    candidates.push(*p);
                }
                Event::Consumer(c) => {
                    let r = candidates.pop();

                    if candidates.contains(c) {
                        saveable.insert(*c);
                    };

                    if r.is_some() {
                        candidates.push(r.unwrap());
                    };
                }
                Event::Caller(_) => {}
            }
        }

        saveable
    }

    /*
        A bufferable register is a saveable register that its lifetime
        does not cross an external call boundary, which can invalidate
        the buffer
    */
    pub fn find_bufferable(&self) -> HashSet<Word> {
        let caller = [
            "rem", "power", "sin", "cos", "tan", "csc", "sec", "cot", "arcsin", "arccos", "arctan",
            "exp", "ln", "log", "root",
        ];

        let mut candidates: Vec<Word> = Vec::new();
        let mut bufferable: HashSet<Word> = HashSet::new();

        for l in self.events.iter() {
            match l {
                Event::Producer(p) => {
                    candidates.push(*p);
                }
                Event::Consumer(c) => {
                    let r = candidates.pop();

                    if candidates.contains(c) {
                        bufferable.insert(*c);
                    };

                    if r.is_some() {
                        candidates.push(r.unwrap());
                    };
                }
                Event::Caller(op) => {
                    if caller.contains(&op.as_str()) {
                        candidates.clear();
                    }
                }
            }
        }

        bufferable
    }
}
