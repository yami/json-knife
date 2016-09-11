extern crate serde_json as json;

#[macro_use]
extern crate lazy_static;

use std::env;
use std::io;
use std::collections::BTreeMap;
use std::iter::Iterator;

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


lazy_static! {
    static ref sBuiltins: BTreeMap<String, FunctionPrototype> = script::make_builtin_funcs();
}


fn action_error(msg: &str) -> Result<(), JkError>
{
    return Err(JkError::Action(String::from(msg)));
}


fn value_error(msg: &str) -> Result<Value, JkError>
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

fn run_array_action(runtime: &mut Runtime, values: &Vec<Value>, action: &Vec<Function>) -> Result<(), JkError>
{
    let var_key = &String::from("_k");
    
    for (i, ref v) in values.iter().enumerate() {
        runtime.var_set(var_key, Value::I64(i as i64));
        
        for func in action {
            run_function(runtime, v, func);
        }
    }

    runtime.var_delete(var_key);

    return Ok(());
}

fn run_object_action(runtime: &mut Runtime, object: &Map<String, Value>, action:&Vec<Function>) -> Result<(), JkError>
{
    let var_key = &String::from("_k");

    for (key, value) in object {
        runtime.var_set(var_key, Value::String(key.clone()));

        for func in action {
            run_function(runtime, value, func);
        }
    }

    runtime.var_delete(var_key);

    return Ok(());
}

fn evaluate_object_index(v: &Value, index: &String) -> Result<Value, JkError>
{
    if let &Value::Object(ref obj) = v {
        if let Some(evalue) = obj.get(index) {
            return Ok(evalue.clone());
        } else {
            return value_error("not found in object");
        }
    } else {
        return value_error("not an object");
    }
}

fn evaluate(runtime: &Runtime, v: &Value, e: &ActionExpr) -> Result<Value, JkError>
{
    match e {
        &ActionExpr::Integer(i) => Ok(Value::I64(i)),
        &ActionExpr::String(ref s) => Ok(Value::String(s.clone())),
        &ActionExpr::ObjectIndex(ref idx) => evaluate_object_index(v, idx),
        &ActionExpr::Variable(ref name) => Ok(runtime.var_get(name)),
    }
}

fn batch_evaluate(runtime: &Runtime, v: &Value, expressions: &Vec<ActionExpr>) -> Result<Vec<Value>, JkError>
{
    let mut evector = Vec::new();
    
    for e in expressions {
        evector.push(try!(evaluate(runtime, v, e)));
    }

    return Ok(evector);
}

fn run_function(runtime: &mut Runtime, v: &Value, func: &Function) -> Result<(), JkError>
{
    if let Some(ref proto) = sBuiltins.get(&func.name) {
        let args = try!(batch_evaluate(runtime, v, &func.args));
        try!((proto.func)(&args));
        return Ok(());
    } else {
        return action_error("function not found");
    }
}


fn run_single_action(runtime: &mut Runtime, v: &Value, action: &Vec<Function>) -> Result<(), JkError>
{
    for func in action {
        try!(run_function(runtime, v, func));
    }

    return Ok(());
}

fn run_action(value: &Value, action: &Vec<Function>) -> Result<(), JkError>
{
    let runtime = &mut Runtime::new();
    
    match value {
        &Value::Array(ref vector) => run_array_action(runtime, vector, action),
        &Value::Object(ref object) => run_object_action(runtime, object, action),
        _ => run_single_action(runtime, value, action),
    }
}

fn execute<R: io::Read>(script: &Script, reader: &mut R) -> Result<(), JkError>
{
    let selector = &script.selector;
    let action = &script.action;

    let mut input = String::new();

    try!(reader.read_to_string(&mut input).map_err(JkError::Io));
    
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
