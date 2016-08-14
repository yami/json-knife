#[macro_use]
extern crate nom;

extern crate serde_json as json;

use std::env;
use std::io;

use std::fs::File;

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
use std::str::FromStr;

use nom::{digit, multispace};

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

#[derive(Debug)]
enum ObjectSelector {
    Wildcard,
    Exact(String),
}

#[derive(Debug)]
struct ArraySlice {
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

        let step = match self.step {
            Some(s) => s as usize,
            None => 1
        };

        // TODO: impl step
        return std::ops::Range{start:start, end:end}
    }
}

#[derive(Debug)]
struct Function {
    name: String,
    args: Vec<String>,
}

#[derive(Debug)]
enum Jop {
    ArraySlice(ArraySlice),
    ArrayIndex(i64),

    Object(ObjectSelector),

    Default(String)
}

#[derive(Debug)]
enum Rop {
    Plain(String),
    Index(String),
    Function(Function),
}

#[derive(Debug)]
struct Script {
    selector: Vec<Jop>,
    action: Vec<Rop>
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

named!(parse<&[u8], Script>,
      chain!(
          selector: parse_selector ~
                    opt!(multispace) ~
                    tag!("@") ~
                    opt!(multispace) ~
          action:   parse_action,
          || { return Script {selector: selector, action: action} }
      )
);

named!(parse_record_selector<&[u8], Rop>,
       chain!(
           tag!(".") ~
               index: map_res!(is_not!(". "), array_to_string),
           || { return Rop::Index(index) } ));

named!(parse_action<&[u8], Vec<Rop> >,
       separated_list!(multispace, parse_record_selector));


named!(parse_selector<&[u8], Vec<Jop> >,
       separated_list!(tag!("."), parse_query));


named!(parse_query<&[u8], Jop>,
       alt!(parse_query_array |
            parse_query_object));

named!(parse_query_array<&[u8], Jop>,
       delimited!(char!('['),
                  alt!(parse_query_index |
                       parse_query_slice_2 |
                       parse_query_slice_3),
                  char!(']')));


named!(parse_query_object<&[u8], Jop>,
       alt!(
           tag!("*") => { |_| Jop::Object(ObjectSelector::Wildcard) }
           | map_res!(is_not!(".{}[]*"), array_to_string) =>  { |s: String| Jop::Object(ObjectSelector::Exact(s)) })); 

fn array_to_sign_value(from: &[u8]) -> Result<i64, JkError>
{
    match from[0] as char {
        '+' => Ok(1),
        '-' => Ok(-1),
        _ => Err(JkError::Query(String::from("hello")))
    }
}

named!(parse_sign<&[u8], i64>,
       map_res!(alt!(tag!("+") | tag!("-")), array_to_sign_value));

named!(parse_i64<i64>,
  map_res!(
    map_res!(
      digit,
      str::from_utf8
    ),
    FromStr::from_str
  )
);
       
named!(parse_signed_i64<&[u8], i64>,
       chain!(sign: opt!(parse_sign) ~
              value: parse_i64,
              ||
              {
                  if let Some(s) = sign {
                      return s * value;
                  } else {
                      return value;
                  }
              } ));

named!(parse_query_index<&[u8], Jop>,
       map_res!(parse_signed_i64,
                |index :i64| -> Result<Jop, Utf8Error> { return Ok(Jop::ArrayIndex(index)) }));

named!(parse_query_slice_2<&[u8], Jop>,
       chain!(
           start: opt!(parse_signed_i64) ~
               char!(':') ~
           end: opt!(parse_signed_i64),
           || { return Jop::ArraySlice(ArraySlice::new(start, end, None))} ));

named!(parse_query_slice_3<&[u8], Jop>,
       chain!(
           start: opt!(parse_signed_i64) ~
                  char!(':') ~
           end: opt!(parse_signed_i64) ~
                  char!(':') ~
           step: opt!(parse_signed_i64),
           || { return Jop::ArraySlice(ArraySlice::new(start, end, step)) } ));

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
            println!("{} ", p);
            return Ok(());
        },
        
        &Rop::Index(ref i) => {
            if let Ok(index) = i.parse::<usize>() {
                println!("{} ", v[index]);
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
            println!("{} ", p);
            return Ok(());
        },

        &Rop::Index(ref i) => {
            if let Some(value) = object.get(i) {
                println!("{} ", value);
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
            println!("{} ", p);
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
        match parse(program.as_bytes()) {
            nom::IResult::Done(i, s) => { execute(&s, &mut io::stdin()); },
            others @ _ => println!("parse error, program={} error={:?}", program, others),
        }
    } else {
        println!("at least one argument must be supplied");
    }
}
