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


use std::string::String;
use std::str;

use json::Value;
use json::Map;


lazy_static! {
    static ref BUILTIN_FUNCS: BTreeMap<String, FunctionPrototype> = script::make_builtin_funcs();
}


fn action_error(msg: &str) -> Result<(), JkError>
{
    return Err(JkError::Action(String::from(msg)));
}


fn value_error(msg: &str) -> Result<Value, JkError>
{
    return Err(JkError::Action(String::from(msg)));
}

fn sop_json_value(value: Value, sop: &Sop) -> Result<Value, JkError>
{
    match value {
        Value::Array(vector) => sop_json_array(vector, sop),
        Value::Object(object) => sop_json_object(object, sop),
        value @ _ => Ok(value.clone()),
    }
}

fn sop_json_array(v: Vec<Value>, sop: &Sop) -> Result<Value, JkError>
{
    match sop {
        &Sop::ArraySlice(ref slice) => Ok(Value::Array(v[slice.to_range(v.len())].to_vec())),
        &Sop::ArrayIndex(index) => Ok(v[index as usize].clone()),
        
        // coercing to foreach
        &Sop::Object(_) => sop_foreach_json_array(v, sop),
    }
}

fn sop_json_object(o: Map<String, Value>, sop: &Sop) -> Result<Value, JkError>
{
    if let &Sop::Object(ref indexer) = sop {
        match indexer {
            &ObjectIndexer::Wildcard => Ok(Value::Object(o)),
            &ObjectIndexer::Exact(ref key) => o.get(key).cloned().ok_or(JkError::Query(String::from("missing"))),
        }
    } else {
        return Err(JkError::Query(String::from("bad object selector")));
    }
}

fn sop_foreach_json_array(vector: Vec<Value>, sop: &Sop) -> Result<Value, JkError>
{
    let mut result = Vec::new();
    
    for v in vector {
        result.push(try!(sop_json_value(v, sop)));
    }
    
    return Ok(Value::Array(result));
}

fn sop_foreach_json_object(object: Map<String, Value>, sop: &Sop) -> Result<Value, JkError>
{
    let mut result = Vec::new();
    
    for (_, v) in object {
        result.push(try!(sop_json_value(v, sop)));
    }
    
    return Ok(Value::Array(result));
}

fn select_json(value: Value, selector: &Selector) -> Result<Value, JkError>
{
    match selector {
        &Selector::ForSelf(ref sop) => sop_json_value(value, sop),
        &Selector::ForEach(ref sop) => {
            match value {
                Value::Array(vector) => sop_foreach_json_array(vector, sop),
                Value::Object(object) => sop_foreach_json_object(object, sop),
                _ => sop_json_value(value, sop),
            }
        },
    }
}

fn run_array_action(runtime: &mut Runtime, values: &Vec<Value>, action: &Vec<Function>) -> Result<(), JkError>
{
    let var_key = &String::from("_k");
    let var_value = &String::from("_v");

    for (i, v) in values.iter().enumerate() {
        runtime.var_set(var_key, Value::I64(i as i64));
        runtime.var_set(var_value, v.clone());
        
        for func in action {
            try!(run_function(runtime, v, func));
        }
    }

    runtime.var_delete(var_key);
    runtime.var_delete(var_value);

    return Ok(());
}

fn run_object_action(runtime: &mut Runtime, object: &Map<String, Value>, action:&Vec<Function>) -> Result<(), JkError>
{
    let var_key = &String::from("_k");
    let var_value = &String::from("_v");

    for (key, value) in object {
        runtime.var_set(var_key, Value::String(key.clone()));
        runtime.var_set(var_value, value.clone());

        for func in action {
            try!(run_function(runtime, value, func));
        }
    }

    runtime.var_delete(var_key);
    runtime.var_delete(var_value);

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

fn evaluate_array_index(v: &Value, index: i64) -> Result<Value, JkError>
{
    if let &Value::Array(ref vector) = v {
        if let Some(evalue) = vector.get(index as usize) {
            return Ok(evalue.clone());
        } else {
            return value_error("out of range");
        }
    } else {
        return value_error("not an array");
    }
}

fn evaluate(runtime: &Runtime, v: &Value, e: &ActionExpr) -> Result<Value, JkError>
{
    match e {
        &ActionExpr::Integer(i) => Ok(Value::I64(i)),
        &ActionExpr::String(ref s) => Ok(Value::String(s.clone())),
        &ActionExpr::ObjectIndex(ref idx) => evaluate_object_index(v, idx),
        &ActionExpr::Variable(ref name) => Ok(runtime.var_get(name)),
        &ActionExpr::ArrayIndex(idx) => evaluate_array_index(v, idx),
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
    if let Some(ref proto) = BUILTIN_FUNCS.get(&func.name) {
        let args = try!(batch_evaluate(runtime, v, &func.args));
        try!((proto.func)(&args));
        return Ok(());
    } else {
        return action_error("function not found");
    }
}


fn run_single_action(runtime: &mut Runtime, v: &Value, action: &Vec<Function>) -> Result<(), JkError>
{
    let var_value = &String::from("_v");
    runtime.var_set(var_value, v.clone());
    
    for func in action {
        try!(run_function(runtime, v, func));
    }

    runtime.var_delete(var_value);

    return Ok(());
}

fn run_foreach_action(value: &Value, action: &Vec<Function>) -> Result<(), JkError>
{
    let runtime = &mut Runtime::new();
    
    match value {
        &Value::Array(ref vector) => run_array_action(runtime, vector, action),
        &Value::Object(ref object) => run_object_action(runtime, object, action),
        _ => run_single_action(runtime, value, action),
    }
}

fn run_forself_action(value: &Value, action: &Vec<Function>) -> Result<(), JkError>
{
    let runtime = &mut Runtime::new();
    return run_single_action(runtime, value, action);
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
        let json_next = try!(select_json(json_curr, s));
        json_curr = json_next;
    }

    match script.mode {
        ActionMode::ForEach => return run_foreach_action(&json_curr, action),
        ActionMode::ForSelf => return run_forself_action(&json_curr, action),
    }
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
