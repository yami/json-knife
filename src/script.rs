extern crate serde_json as json;

use json::Value;
use json::Map;


use std;
use std::io;
use std::collections::BTreeMap;
use std::fmt;


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
pub enum ActionExpr {
    Integer(i64),
    String(String),
    ObjectIndex(String),
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub args: Vec<ActionExpr>,
}

#[derive(Debug)]
pub enum Jop {
    ArraySlice(ArraySlice),
    ArrayIndex(i64),

    Object(ObjectSelector),

    Default(String)
}


#[derive(Debug)]
pub struct Script {
    pub selector: Vec<Jop>,
    pub action: Vec<Function>
}


#[derive(Debug)]
pub enum JkError {
    Io(io::Error),
    Parse(json::Error),
    Query(String),
    Action(String),
}


// function prototypes
pub struct FunctionPrototype {
    pub func: fn (&Vec<Value>) -> Result<Value, JkError>,
}

impl fmt::Debug for FunctionPrototype {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FunctionPrototype")
    }
}

fn builtin_print(args: &Vec<Value>) -> Result<Value, JkError>
{
    for a in args {
        print!("{} ", a);
    }

    print!("\n");
    
    return Ok(Value::Null);
}


pub fn make_builtin_funcs() -> BTreeMap<String, FunctionPrototype>
{
    let mut m = BTreeMap::new();

    m.insert(String::from("p"), FunctionPrototype { func: builtin_print });

    return m;
}
