extern crate serde_json as json;

use std::env;
use std::io;

mod script;
mod parse;

use parse::script;
use script::*;

// selector convert a json value to another json value
// action prints records in a json value in tabluar form
// 
// To print result = [ {"a": 1, "b": 2, "c": 3 }, {"a": 4, "b": 5, "c": 6 }, {"a": 6, "b": 7, "c": 8}]
//     for k, v in enumerate(result):
//         record-print()
//
// 
// record-print a json value
//
// 1. Object
//    record = {"a": 1, "b": 2, "c": 3}
//    
//    p .a .b       <<< print r['a'], r['b']
//
//    p .values()   <<< print r['a'], r['b'], r['c']
//
//    p .keys()     <<< print 'a', 'b', 'c'
//
//    p .join(':')  <<< print "'a':r['a']", "'b':r['b']", "'c':r['c']"
//
//
// 2. Array
//    record = [1, 2, 3, 4]
//
//    p .0 .1       <<< print r[0], r[1]
//    p .values()   <<< print r[0], r[1], r[2], r[3]
//    p .keys()     <<< print 0, 1, 2, 3
//    p .join(':')  <<< print 0:r[0], 1:r[1], 2:r[2], 3:r[3]
//

use std::string::String;
use std::str::Utf8Error;
use std::str;

use json::Value;
use json::Map;

#[derive(Debug)]
enum JkError {
    Io(io::Error),
    Parse(json::Error),
    Query(String),
    Action(String),
}

fn action_error(msg: &str) -> Result<(), JkError>
{
    return Err(JkError::Action(String::from(msg)));
}



fn array_to_op_string(from: &[u8]) -> Result<Jop, Utf8Error>
{
    match str::from_utf8(from) {
        Ok(value) => Ok(Jop::Default(String::from(value))),
        Err(e) => Err(e)
    }
}

fn array_to_string(from: &[u8]) -> Result<String, Utf8Error>
{
    match str::from_utf8(from) {
        Ok(value) => Ok(String::from(value)),
        Err(e) => Err(e)
    }
}


fn array_to_sign_value(from: &[u8]) -> Result<i64, JkError>
{
    match from[0] as char {
        '+' => Ok(1),
        '-' => Ok(-1),
        _ => Err(JkError::Query(String::from("hello")))
    }
}

fn select_json_array(v: Vec<Value>, query: &Jop) -> Result<Value, JkError>
{
    match query {
        &Jop::ArraySlice(ref slice) => Ok(Value::Array(v[slice.to_range(v.len())].to_vec())),
        &Jop::ArrayIndex(index) => Ok(v[index as usize].clone()),
        _ => Err(JkError::Query(String::from("bad array selector")))
    }
}

fn select_json_object(o: Map<String, Value>, query: &Jop) -> Result<Value, JkError>
{
    if let &Jop::Object(ref selector) = query {
        match selector {
            &ObjectSelector::Wildcard => Ok(Value::Object(o)),
            &ObjectSelector::Exact(ref key) => o.get(key).cloned().ok_or(JkError::Query(String::from("missing"))),
        }
    } else {
        return Err(JkError::Query(String::from("bad object selector")));
    }
}

fn select_json_value(node: Value, query: &Jop) -> Result<Value, JkError>
{
    match node {
        Value::Array(vector) => select_json_array(vector, query),
        Value::Object(object) => select_json_object(object, query),
        value @ _ => Ok(value.clone()),
    }
}

fn run_array_action(v: &Vec<Value>, rop: &Rop) -> Result<(), JkError>
{
    match rop {
        &Rop::Plain(ref p) => {
            print!("{} ", p);
            return Ok(());
        },
        
        &Rop::Index(ref i) => {
            if let Ok(index) = i.parse::<usize>() {
                print!("{} ", v[index]);
                return Ok(());
            } else {
                return action_error("run_array_action fail to parse index as usize");
            }
        },
        
        &Rop::Function(ref f) => {
            return action_error("run_array_action function not supported");
        },
    }

    return action_error("unknown rop");
}

fn run_object_action(object: &Map<String, Value>, rop: &Rop) -> Result<(), JkError>
{
    match rop {
        &Rop::Plain(ref p) => {
            print!("{} ", p);
            return Ok(());
        },

        &Rop::Index(ref i) => {
            if let Some(value) = object.get(i) {
                print!("{} ", value);
                return Ok(());
            } else {
                return action_error("run_object_action fail to parse index as usize");
            }
        },

        &Rop::Function(ref f) => {
            return action_error("run_object_action function not supported");
        },
    }
}

fn run_single_action(value: &Value, rop: &Rop) -> Result<(), JkError>
{
    match rop {
        &Rop::Plain(ref p) => {
            print!("{} ", p);
            return Ok(());
        },

        &Rop::Index(ref i) => {
            return action_error("run_object_action index not supported");
        },
        
        &Rop::Function(ref f) => {
            return action_error("run_single_action function not supported");
        },

    }
}

fn run_action(value: &Value, action: &Vec<Rop>) -> Result<(), JkError>
{
    for rop in action {
        let result = match value {
            &Value::Array(ref vector) => run_array_action(vector, rop),
            &Value::Object(ref object) => run_object_action(object, rop),
            _ => run_single_action(value, rop),
        };

        if result.is_err() {
            return result;
        }
    }

    return Ok(());
}

fn execute<R: io::Read>(script: &Script, reader: &mut R) -> Result<(), JkError>
{
    let selector = &script.selector;
    let action = &script.action;

    let mut input = String::new();

    try!(reader.read_to_string(&mut input).map_err(JkError::Io));

    println!("input: {:?}", input);

    let json_root: Value = try!(json::from_str(&input).map_err(JkError::Parse));
    let mut json_curr = json_root;

    for s in selector {
        let json_next = try!(select_json_value(json_curr, s));
        json_curr = json_next;
    }

    println!("result: {:?}", json_curr);

    return run_action(&json_curr, action);
}

fn main() {
    if let Some(program) = env::args().nth(1) {
        match script(&program) {
            Ok(s) => { execute(&s, &mut io::stdin()).unwrap(); },
            Err(e) => println!("parse error, program={} error={:?}", program, e),
        }
    } else {
        println!("at least one argument must be supplied");
    }
}
