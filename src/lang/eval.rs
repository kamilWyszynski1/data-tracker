use fancy_regex::Regex;
use lazy_static::lazy_static;
use reqwest::blocking;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

/// Our own wrapped Result type.
pub type EvalResult<T> = std::result::Result<T, EvalError>;

#[derive(Debug, Clone)]
pub struct EvalError {
    eval_part: String,
    msg: String,
    comment: String,
}

impl EvalError {
    fn new(eval_part: String, msg: String) -> Self {
        EvalError {
            eval_part,
            msg,
            comment: String::new(),
        }
    }

    /// Returns cloned version of EvalError with set comment.
    fn with_comment(&mut self, comment: String) -> EvalError {
        self.comment = comment;
        self.clone()
    }
}

impl Error for EvalError {}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.comment.is_empty() {
            write!(f, "(eval_part: {}, msg: {})", self.eval_part, self.msg)
        } else {
            write!(
                f,
                "(eval_part: {}, msg: {}, comment: {})",
                self.eval_part, self.msg, self.comment
            )
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Variable {
    None,
    Bool(bool),
    Int(isize),
    Float(f32),
    String(String),
    Vector(Vec<Variable>),
    Object(HashMap<String, Variable>),
    Json(Value),
}

fn serde_value_to_variable(v: Value) -> Variable {
    if v.is_boolean() {
        return Variable::Bool(v.as_bool().unwrap());
    } else if v.is_string() {
        return Variable::String(v.as_str().unwrap().to_string());
    } else if v.is_i64() {
        return Variable::Int(v.as_i64().unwrap() as isize);
    } else if v.is_f64() {
        return Variable::Float(v.as_f64().unwrap() as f32);
    } else if v.is_array() {
        let vec: Vec<Variable> = v
            .as_array()
            .unwrap()
            .iter()
            .map(|v| serde_value_to_variable(v.clone()))
            .collect();
        return Variable::Vector(vec);
    } else if v.is_object() {
        return value_object_to_variable_object(v);
    }
    Variable::None
}

pub fn value_object_to_variable_object(v: Value) -> Variable {
    let m: HashMap<String, Variable> = v
        .as_object()
        .unwrap()
        .into_iter()
        .map(|(k, v)| (k.clone(), serde_value_to_variable(v.clone())))
        .collect();
    Variable::Object(m)
}

impl Variable {
    pub fn extract(&self, f: &str) -> Result<Variable, &'static str> {
        match *self {
            Variable::None
            | Variable::Bool(_)
            | Variable::Int(_)
            | Variable::Float(_)
            | Variable::String(_) => Err("cannot extract"),
            Variable::Vector(ref vec) => {
                let inx = f.parse::<usize>().or_else(|_| Err("invalid index"))?;
                if vec.len() < inx - 1 {
                    return Err("index out of range");
                }
                Ok(vec[inx].clone())
            }
            Variable::Object(ref obj) => match obj.get(f) {
                Some(v) => Ok(v.clone()),
                None => Err("json does not have this field"),
            },
            Variable::Json(ref jsn) => match jsn.get(f) {
                Some(v) => Ok(serde_value_to_variable(v.clone())),
                None => Err("json does not have this field"),
            },
        }
    }
}

/// Evaluates single step recursively.
pub fn evaluate(
    step: String,
    variables: &mut HashMap<String, Variable>,
) -> EvalResult<Option<Variable>> {
    if step.matches('(').count() == 0 {
        dbg!("returning string variable: {}", step.clone());
        return Ok(Some(Variable::String(step.replace(' ', ""))));
    }

    let split = step.split_once('(');
    let (function, mut rest) = split.unwrap();
    rest = rest.strip_suffix(')').unwrap();
    dbg!(rest);
    match function.to_lowercase().replace(' ', "").as_str() {
        "define" => handle_define(rest, variables),
        "int" => Ok(Some(int(rest))),
        "float" => Ok(Some(float(rest))),
        "json" => Ok(Some(json(rest))),
        "vec" => handle_vec_parameters(rest, variables),
        "object" => Ok(Some(object(rest))),
        "extract" => handle_extract(rest, variables),
        "get" => get(rest),
        _ => Ok(None),
    }
}

/// Evaluates parameter and writes it to variables state.
fn handle_define(
    s: &str,
    variables: &mut HashMap<String, Variable>,
) -> EvalResult<Option<Variable>> {
    let (name, arg) = double_argument(s);
    let step = arg.to_string();
    let value = evaluate(arg.to_string(), variables)?
        .ok_or(EvalError::new(step, String::from("value is none")))?; // evaluate argument.
    define(variables, name, value);
    Ok(None)
}

/// Takes parameters, evaluates all of them and creates vector.
fn handle_vec_parameters(
    parameters_str: &str,
    variables: &mut HashMap<String, Variable>,
) -> EvalResult<Option<Variable>> {
    // regex will be parsed only once.
    lazy_static! {
        static ref VECTOR_SPLIT: Regex = Regex::new(r",(?![^JSON(]*\))").unwrap();
    }

    let mut values: Vec<Variable> = vec![];
    let positions = VECTOR_SPLIT.find_iter(parameters_str);

    let mut left_bound = 0;
    for p in positions {
        let mtch = p.or_else(|_| {
            Err(EvalError::new(
                String::from(parameters_str),
                String::from("could not unwrap match"),
            ))
        })?;
        dbg!(mtch);
        values.push(
            evaluate(
                parameters_str[left_bound..mtch.end() - 1].to_string(),
                variables,
            )
            .unwrap()
            .unwrap(),
        );
        left_bound = mtch.end();
    }
    values.push(
        evaluate(parameters_str[left_bound..].to_string(), variables)
            .unwrap()
            .unwrap(),
    );

    Ok(Some(vector(values)))
}

/// Extracts given field/index from Variable.
fn handle_extract(
    s: &str,
    variables: &mut HashMap<String, Variable>,
) -> EvalResult<Option<Variable>> {
    let (name, arg) = double_argument(s);

    let e = extract(variables, name, arg.replace(' ', "").as_str());
    match e {
        Err(e) => Err(EvalError::new(String::from(s), String::from(e))),
        Ok(v) => Ok(Some(v)),
    }
}

/// Adds defined variable to state.
fn define(variables: &mut HashMap<String, Variable>, variable_name: &str, value: Variable) {
    variables.insert(variable_name.to_string(), value);
}

/// Returns Variable::Int from &str.
fn int(value: &str) -> Variable {
    Variable::Int(value.parse().unwrap())
}

/// Returns Variable::Float from &str.
fn float(value: &str) -> Variable {
    Variable::Float(value.parse::<f32>().unwrap())
}

/// Returns Variable::Json from &str.
fn json(value: &str) -> Variable {
    Variable::Json(serde_json::from_str(value).unwrap())
}

/// Returns Variable::Array from given values.
fn vector(values: Vec<Variable>) -> Variable {
    Variable::Vector(values)
}

/// Returns Variable::Object from &str.
fn object(value: &str) -> Variable {
    let v: Value = serde_json::from_str(value).unwrap();
    assert!(v.is_object());
    value_object_to_variable_object(v)
}

/// Extracts field from variable.
fn extract(
    variables: &HashMap<String, Variable>,
    variable_name: &str,
    field_name: &str,
) -> Result<Variable, &'static str> {
    match variables.get(variable_name) {
        Some(var) => match var.extract(field_name) {
            Ok(field) => Ok(field),
            Err(err) => Err(err),
        },
        None => Err("variable not defined"),
    }
}

/// Performs blocking request to wanted url and saves result as Variable::Json.
fn get(url: &str) -> EvalResult<Option<Variable>> {
    let body: Value = blocking::get(url).unwrap().json().unwrap();
    Ok(Some(Variable::Json(body)))
}

/// Assures that there's only two arguments and returns them.
fn double_argument(s: &str) -> (&str, &str) {
    s.split_once(',').unwrap()
}
