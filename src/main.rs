#[macro_use]
extern crate nom;

extern crate serde_json as json;

use std::env;
use std::io;

use std::fs::File;


use std::string::String;
use std::str::Utf8Error;
use std::str;
use std::str::FromStr;

use nom::{digit};

use json::Value;
use json::Map;

#[derive(Debug)]
enum JkError {
    Io(io::Error),
    Parse(json::Error),
    Query(String),
}

#[derive(Debug)]
enum ObjectSelector {
    Wildcard,
    Exact(String),
}

#[derive(Debug)]
enum Op {
    ArraySlice { start: Option<i64>,
                 end: Option<i64>,
                 step: Option<i64>
    },

    Object(ObjectSelector),

    Default(String)
}

#[derive(Debug)]
struct Script {
    selector: Vec<Op>,
    action: String
}


fn array_to_op_string(from: &[u8]) -> Result<Op, Utf8Error>
{
    match str::from_utf8(from) {
        Ok(value) => Ok(Op::Default(String::from(value))),
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
          action: parse_action,
          || { return Script {selector: selector, action: action} }
      )
);

named!(parse_action<&[u8], String>,
       map_res!(delimited!(char!('{'), is_not!("}"), char!('}')),
                array_to_string));


named!(parse_selector<&[u8], Vec<Op> >,
       separated_list!(tag!("."), parse_query));


named!(parse_query<&[u8], Op>,
       alt!(parse_query_array |
            parse_query_object));

named!(parse_query_array<&[u8], Op>,
       delimited!(char!('['),
                  alt!(parse_query_slice_2 |
                       parse_query_slice_3),
                  char!(']')));


named!(parse_query_object<&[u8], Op>,
       alt!(
           tag!("*") => { |_| Op::Object(ObjectSelector::Wildcard) }
           | map_res!(is_not!(".{}[]*"), array_to_string) =>  { |s: String| Op::Object(ObjectSelector::Exact(s)) })); 

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

named!(parse_query_slice_2<&[u8], Op>,
       chain!(
           start: opt!(parse_signed_i64) ~
               char!(':') ~
           end: opt!(parse_signed_i64),
           || { return Op::ArraySlice { start: start, end: end, step: None} }));

named!(parse_query_slice_3<&[u8], Op>,
       chain!(
           start: opt!(parse_signed_i64) ~
                  char!(':') ~
           end: opt!(parse_signed_i64) ~
                  char!(':') ~
           step: opt!(parse_signed_i64),
           || { return Op::ArraySlice{ start: start, end: end, step: step} } ));

fn select_json_array(v: Vec<Value>, query: &Op) -> Result<Value, JkError>
{
    println!("in select array: {:?}, Op: {:?}", v, query);
    return Ok(v[0].clone());
}

fn select_json_object(o: Map<String, Value>, query: &Op) -> Result<Value, JkError>
{
    if let &Op::Object(ref selector) = query {
        match selector {
            &ObjectSelector::Wildcard => Ok(Value::Object(o)),
            &ObjectSelector::Exact(ref key) => o.get(key).cloned().ok_or(JkError::Query(String::from("missing"))),
        }
    } else {
        return Err(JkError::Query(String::from("bad selector")));
    }
}

fn select_json_value(node: Value, query: &Op) -> Result<Value, JkError>
{
    match node {
        Value::Null => Ok(Value::Null),
        Value::Bool(v) => Ok(Value::Bool(v)),
        Value::I64(v) => Ok(Value::I64(v)),
        Value::U64(v) => Ok(Value::U64(v)),
        Value::F64(v) => Ok(Value::F64(v)),
        Value::String(v) => Ok(Value::String(v)),
        Value::Array(v) => select_json_array(v, query),
        Value::Object(v) => select_json_object(v, query),
        
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

    return Ok(());
}

fn main() {
    if let Some(program) = env::args().nth(1) {
        if let nom::IResult::Done(i, s) = parse(program.as_bytes()) {
            let mut f = File::open("test.json").unwrap();
            execute(&s, &mut f);
        } else {
            println!("parse error!");
        }
    } else {
        println!("at least one argument must be supplied");
    }
}
