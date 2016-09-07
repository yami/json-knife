use std;

#[derive(Debug)]
pub enum ObjectSelector {
    Wildcard,
    Exact(String),
}

#[derive(Debug)]
pub struct ArraySlice {
    start: Option<i64>,
    end: Option<i64>,
    step: Option<i64>
}

impl ArraySlice {
    pub fn new(start: Option<i64>, end: Option<i64>, step: Option<i64>) -> ArraySlice {
        ArraySlice {
            start: start,
            end: end,
            step: step,
        }
    }

    pub fn to_range(&self, len: usize) -> std::ops::Range<usize> {
        // TODO: handle negative
        let start = match self.start {
            Some(s) => s as usize,
            None => 0,
        };

        let end = match self.end {
            Some(e) => e as usize,
            None => len,
        };

        // TODO: impl step
        return std::ops::Range{start:start, end:end}
    }
}

#[derive(Debug)]
pub struct Function {
    name: String,
    args: Vec<String>,
}

#[derive(Debug)]
pub enum Jop {
    ArraySlice(ArraySlice),
    ArrayIndex(i64),

    Object(ObjectSelector),

    Default(String)
}

#[derive(Debug)]
pub enum Rop {
    Plain(String),
    Index(String),
    Function(Function),
}

#[derive(Debug)]
pub struct Script {
    pub selector: Vec<Jop>,
    pub action: Vec<Rop>
}
