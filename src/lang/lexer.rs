use super::{engine::Engine, variable::Variable};
use crate::error::types::{Error, Result};
use crate::lang::{eval::EvalForest, variable::value_object_to_variable_object};
use core::panic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::VecDeque;
use std::{
    collections::HashMap,
    fmt::{self, Display},
};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
/// Enum for Node type.
pub enum NodeEnum {
    None,
    Keyword(Keyword), // Keyword is a supported function.
    Var(String),      // Variable name or "default" evaluation of variable which is String.
}

impl Default for NodeEnum {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Default)]
/// Structure holds meta information about evaluation process.
/// Can be handy for passing map placeholder info.
pub struct EvalMetadata {
    pub mapped_variable: Option<Variable>,
}

#[derive(Debug, Default)]
/// Quasi implementation of stack. It'll track calls of subtrees.
/// Stack will have information about root, current call and wether we should break or not.
pub struct Stack {
    stack: VecDeque<String>,
    pub should_break: bool,
}

impl Stack {
    fn push(&mut self, subtree: String) {
        self.stack.push_back(subtree)
    }

    fn pop(&mut self) {
        self.stack.pop_back();
        if self.stack.len() == 0 {
            self.should_break = false;
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone, Serialize, Deserialize)]
/// Node represents single node in lexer chain.
/// Struct contains value which is type of Node -> var or keyword.
/// Vector of nodes are all params that were passed to keyword function and will
/// be evaluated during Node evaluation.
pub struct Node {
    pub value: NodeEnum,
    pub nodes: Vec<Node>,
}

impl Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

impl Node {
    /// Returns default value for Node.
    pub fn default() -> Self {
        Node {
            value: NodeEnum::default(),
            nodes: vec![],
        }
    }

    /// Creates new Node with NodeEnum::Keyword type.
    fn new_keyword(keyword: Keyword) -> Self {
        Node {
            value: NodeEnum::Keyword(keyword),
            nodes: vec![],
        }
    }

    /// Creates new Node with NodeEnum::Var type.
    fn new_var(var: String) -> Self {
        Node {
            value: NodeEnum::Var(var),
            nodes: vec![],
        }
    }

    /// Adds nodes to node.
    fn push(&mut self, pt: Node) {
        self.nodes.push(pt)
    }

    /// Appends node to nodes and return Self.
    fn append(&mut self, n: Node) -> Self {
        self.push(n);
        self.clone()
    }

    /// Serializes whole tree to json string.
    fn to_string(&self) -> Result<String> {
        serde_json::to_string(self)
            .map_err(|err| Error::new_eval_internal(String::from("to_string"), err.to_string()))
    }

    /// Loads tree from json string.
    fn from_string(s: &str) -> Result<Self> {
        serde_json::from_str::<Self>(s)
            .map_err(|err| Error::new_eval_internal(String::from("from_string"), err.to_string()))
    }

    fn change_special_function_placeholder(&mut self, replacement: Self) {
        if self.value == NodeEnum::Var(String::from("X")) {
            *self = replacement.clone()
        }
        for i in 0..self.nodes.len() {
            self.nodes[i].change_special_function_placeholder(replacement.clone())
        }
    }

    pub fn start_evaluation(
        &self,
        variables: &mut HashMap<String, Variable>,
        subtrees: &HashMap<String, Vec<Node>>,
    ) -> Result<Variable> {
        let mut subtree_stack = Stack::default();
        let mut metadata = EvalMetadata::default();
        self.eval(variables, subtrees, &mut subtree_stack, &mut metadata)
    }

    /// Evaluates whole tree to a single Variable.
    /// Function can be chained with each other as shown below:
    ///   "VEC(1,BOOL(true),3,FLOAT(4.0))"
    /// this function will go trough tree created from that declaration
    /// and evaluate root node and all of nodes below in order to return
    /// single Variable as a result.
    pub fn eval(
        &self,
        variables: &mut HashMap<String, Variable>,
        subtrees: &HashMap<String, Vec<Node>>,
        stack: &mut Stack,
        metadata: &mut EvalMetadata,
    ) -> Result<Variable> {
        match self.value {
            NodeEnum::None => Ok(Variable::None),
            NodeEnum::Keyword(ref keyword) => {
                if stack.should_break {
                    stack.pop(); // pop latest subtree from stack.
                    return Ok(Variable::None);
                }

                // must be checked before further evaluation.
                if let Keyword::If = keyword {
                    if !if_check(&self.nodes, variables, subtrees)? {
                        return Ok(Variable::None);
                    }
                }

                match keyword {
                    Keyword::If => {
                        if !if_check(&self.nodes, variables, subtrees)? {
                            return Ok(Variable::None);
                        }
                    }
                    Keyword::Map => {
                        return map_function(&self.nodes, variables, subtrees, metadata);
                    }
                    Keyword::Filter => return filter(&self.nodes, variables, subtrees, metadata),
                    _ => {}
                }

                let nodes = self
                    .nodes
                    .iter()
                    .map(|n| n.eval(variables, subtrees, stack, metadata))
                    .collect::<Result<Vec<Variable>>>()?;

                // check number of arguments.
                keyword.check_arguments_count(&nodes)?;

                match keyword {
                    Keyword::Bool => bool(&nodes),
                    Keyword::Int => int(&nodes),
                    Keyword::Float => float(&nodes),
                    Keyword::Add => add(&nodes),
                    Keyword::Sub => sub(&nodes),
                    Keyword::Div => div(&nodes),
                    Keyword::Mult => mult(&nodes),
                    Keyword::Vec => Ok(Variable::Vector(nodes)),
                    Keyword::Extract => extract(&nodes),
                    Keyword::Define => define(&nodes, variables),
                    Keyword::Get => get(&nodes, variables),
                    Keyword::Json => json(&nodes),
                    Keyword::Object => object(&nodes),
                    Keyword::HTTP => http(&nodes),
                    Keyword::Log => log(&nodes),
                    Keyword::RunSubtree => {
                        run_subtree(&nodes, variables, subtrees, stack, metadata)
                    }
                    Keyword::If => if_return(&nodes),
                    Keyword::Eq => eq(&nodes),
                    Keyword::Neq => neq(&nodes),
                    Keyword::Break => {
                        break_function(stack);
                        Ok(Variable::None)
                    }
                    _ => panic!("should not be reached"),
                }
            }
            NodeEnum::Var(ref var) => Ok(Variable::String(var.clone())),
        }
    }
}

fn bool(nodes: &[Variable]) -> Result<Variable> {
    Ok(Variable::Bool(parse_single_param(nodes).map_err(
        |err| Error::new_eval_internal(String::from("bool"), err.to_string()),
    )?))
}

fn int(nodes: &[Variable]) -> Result<Variable> {
    Ok(Variable::Int(parse_single_param(nodes).map_err(|err| {
        Error::new_eval_internal(String::from("bool"), err.to_string())
    })?))
}

fn float(nodes: &[Variable]) -> Result<Variable> {
    Ok(Variable::Float(parse_single_param(nodes)?))
}

fn add(nodes: &[Variable]) -> Result<Variable> {
    println!("add: {:?}", nodes);

    let mut is_float = false;
    let mut sum: f32 = 0.;

    for n in nodes.iter() {
        match n.clone() {
            Variable::Float(f) => {
                sum += f;
                is_float = true;
            }
            Variable::Int(i) => sum += i as f32,
            _ => {
                return Err(Error::new_eval_invalid_type(
                    String::from("add"),
                    format!("{:?}", n),
                    String::from("Variable::Float or Variable::Int"),
                ));
            }
        }
    }
    if is_float {
        return Ok(Variable::Float(sum));
    }
    Ok(Variable::Int(sum as isize))
}

/// Subtracts one Variable from another.
fn sub(nodes: &[Variable]) -> Result<Variable> {
    let mut iter = nodes.iter();
    let v1 = iter.next().unwrap();
    let v2 = iter.next().unwrap();

    match v1 {
        Variable::Float(f) => {
            if let Variable::Float(f2) = v2 {
                Ok(Variable::Float(f - f2))
            } else {
                Err(Error::new_eval_invalid_type(
                    String::from("sub"),
                    type_of(v2),
                    String::from("Variable::Float"),
                ))
            }
        }
        Variable::Int(i) => {
            if let Variable::Int(i2) = v2 {
                Ok(Variable::Int(i - i2))
            } else {
                Err(Error::new_eval_invalid_type(
                    String::from("sub"),
                    type_of(v2),
                    String::from("Variable::Int"),
                ))
            }
        }
        _ => Err(Error::new_eval_invalid_type(
            String::from("sub"),
            type_of(v1),
            String::from("Variable::Float or Variable::Int"),
        )),
    }
}
/// Divides one Variable by another.
fn div(nodes: &[Variable]) -> Result<Variable> {
    let mut iter = nodes.iter();
    let v1 = iter.next().unwrap();
    let v2 = iter.next().unwrap();

    match v1 {
        Variable::Float(f) => {
            if let Variable::Float(f2) = v2 {
                Ok(Variable::Float(f / f2))
            } else {
                Err(Error::new_eval_invalid_type(
                    String::from("add"),
                    type_of(v2),
                    String::from("Variable::Float"),
                ))
            }
        }
        Variable::Int(i) => {
            if let Variable::Int(i2) = v2 {
                Ok(Variable::Int(i / i2))
            } else {
                Err(Error::new_eval_invalid_type(
                    String::from("add"),
                    type_of(v2),
                    String::from("Variable::Int"),
                ))
            }
        }
        _ => Err(Error::new_eval_invalid_type(
            String::from("add"),
            type_of(v1),
            String::from("Variable::Float or Variable::Int"),
        )),
    }
}

/// Multiplies one Variable by another.
fn mult(nodes: &[Variable]) -> Result<Variable> {
    let mut iter = nodes.iter();
    let v1 = iter.next().unwrap();
    let v2 = iter.next().unwrap();

    match v1 {
        Variable::Float(f) => {
            if let Variable::Float(f2) = v2 {
                Ok(Variable::Float(f * f2))
            } else {
                Err(Error::new_eval_invalid_type(
                    String::from("add"),
                    type_of(&v2),
                    String::from("Variable::Float"),
                ))
            }
        }
        Variable::Int(i) => {
            if let Variable::Int(i2) = v2 {
                Ok(Variable::Int(i * i2))
            } else {
                Err(Error::new_eval_invalid_type(
                    String::from("add"),
                    type_of(&v2),
                    String::from("Variable::Int"),
                ))
            }
        }
        _ => Err(Error::new_eval_invalid_type(
            String::from("add"),
            type_of(&v1),
            String::from("Variable::Float or Variable::Int"),
        )),
    }
}
fn type_of<T>(_: &T) -> String {
    std::any::type_name::<T>().to_string()
}

// Extracts field/index from wanted Variable.
fn extract(nodes: &[Variable]) -> Result<Variable> {
    let mut iter = nodes.iter();
    let v1 = iter.next().unwrap();
    let v2 = iter.next().unwrap();
    let deep = iter
        .next()
        .and_then(|f| Some(f.is_true()))
        .unwrap_or_default(); // optional argument

    v1.extract(v2, deep)
}

// Defines new variable and writes it to a state.
fn define(nodes: &[Variable], state: &mut HashMap<String, Variable>) -> Result<Variable> {
    debug!("define - nodes: {:?}", nodes);

    let mut iter = nodes.iter();
    let v1 = iter.next().unwrap();
    let v2 = iter.next().unwrap().to_owned();

    if let Variable::String(s) = v1 {
        state.insert(s.to_string(), v2);
    } else {
        return Err(Error::new_eval_invalid_type(
            String::from("define"),
            type_of(v1),
            String::from("Variable::String"),
        ));
    }

    Ok(Variable::None)
}

// Returns declared variable.
fn get(nodes: &[Variable], state: &HashMap<String, Variable>) -> Result<Variable> {
    let v = parse_single_param::<String>(nodes)
        .map_err(|err| Error::new_eval_internal(String::from("bool"), err.to_string()))?;

    let g = state.get(&v).ok_or_else(|| {
        Error::new_eval_internal(String::from("get"), format!("variable: {} not found", v))
    })?;
    Ok(g.clone())
}

// Returns Variable::Object parsed from json-like string.
fn object(nodes: &[Variable]) -> Result<Variable> {
    let v = parse_single_param::<String>(nodes)
        .map_err(|err| Error::new_eval_internal(String::from("bool"), err.to_string()))?;

    let obj: Value = serde_json::from_str(&v)
        .map_err(|err| Error::new_eval_internal(String::from("object"), err.to_string()))?;
    if !obj.is_object() {
        return Err(Error::new_eval_internal(
            String::from("object"),
            String::from("json Value is not an object"),
        ));
    }
    Ok(value_object_to_variable_object(obj))
}

// Returns Variable::Json.
fn json(nodes: &[Variable]) -> Result<Variable> {
    let v = parse_single_param::<String>(nodes)
        .map_err(|err| Error::new_eval_internal(String::from("object"), err.to_string()))?;
    let obj: Value = serde_json::from_str(&v)
        .map_err(|err| Error::new_eval_internal(String::from("object"), err.to_string()))?;
    Ok(Variable::Json(obj))
}

// Performs GET http request, returns Variable::Json.
fn http(nodes: &[Variable]) -> Result<Variable> {
    let url = parse_single_param::<String>(nodes)
        .map_err(|err| Error::new_eval_internal(String::from("http"), err.to_string()))?;

    let body = reqwest::blocking::get(url)
        .map_err(|err| Error::new_eval_internal(String::from("http"), err.to_string()))?
        .json()
        .map_err(|err| Error::new_eval_internal(String::from("http"), err.to_string()))?;
    Ok(Variable::Json(body))
}

fn log(nodes: &[Variable]) -> Result<Variable> {
    info!("value of nods: {:?}", nodes);
    Ok(Variable::None)
}

/// Max times when 'RunSubtree' can be called in a loop.
const MAX_SUBTREE_STACK: usize = 100;

/// Performs subtree run on each elements of Vector/Object, other types are not supported.
/// Input and Output type of Variable must match.
fn run_subtree(
    nodes: &[Variable],
    variables: &mut HashMap<String, Variable>,
    subtrees: &HashMap<String, Vec<Node>>,
    stack: &mut Stack,
    metadata: &mut EvalMetadata,
) -> Result<Variable> {
    if stack.stack.len() == MAX_SUBTREE_STACK {
        return Err(Error::new_eval_internal(
            String::from("run_subtree"),
            String::from("stack overflow"),
        ));
    }

    let mut iter = nodes.iter();
    let subtree_name = iter.next().unwrap().to_str()?;

    debug!("run_subtree - running {} subtree", subtree_name);

    subtrees
        .get(subtree_name)
        .ok_or(Error::new_eval_internal(
            String::from("run_subtree_for_each"),
            format!("invalid {} subtree", subtree_name),
        ))
        .and_then(|subtree| {
            stack.push(subtree_name.to_string()); // add subtree call to
            fire_subtree(subtree, variables, subtrees, stack, metadata)
        })
        .map(|_| Variable::None)
}

pub fn fire_subtree(
    roots: &[Node],
    variables: &mut HashMap<String, Variable>,
    subtrees: &HashMap<String, Vec<Node>>,
    stack: &mut Stack,
    metadata: &mut EvalMetadata,
) -> Result<()> {
    for root in roots {
        root.eval(variables, subtrees, stack, metadata)?;
    }
    Ok(())
}

/// Function checks *first* element of nodes if can be evaluated to 'true'.
/// If so, second arguments as some operation will be run.
fn if_check(
    nodes: &[Node],
    state: &mut HashMap<String, Variable>,
    subtrees: &HashMap<String, Vec<Node>>,
) -> Result<bool> {
    debug!("if_function - nodes: {:?}", nodes);
    // eval first node which is conditional value.
    nodes[0]
        .start_evaluation(state, subtrees)
        .and_then(|v| Ok(v.is_true()))
}

fn break_function(stack: &mut Stack) {
    stack.should_break = true
}

/// Returns value evaluated after 'if_check'.
fn if_return(nodes: &[Variable]) -> Result<Variable> {
    Ok(nodes[1].clone())
}

fn eq(nodes: &[Variable]) -> Result<Variable> {
    Ok(Variable::Bool(nodes[0].equals(&nodes[1])))
}

fn neq(nodes: &[Variable]) -> Result<Variable> {
    Ok(Variable::Bool(!nodes[0].equals(&nodes[1])))
}

fn map_function(
    nodes: &[Node],
    variables: &mut HashMap<String, Variable>,
    subtrees: &HashMap<String, Vec<Node>>,
    metadata: &mut EvalMetadata,
) -> Result<Variable> {
    assert_eq!(nodes.len(), 2);
    let mapped_variable = match nodes[0].start_evaluation(variables, subtrees) {
        Ok(v) => Ok(v),
        Err(e) => match &metadata.mapped_variable {
            Some(v) => Ok(v.clone()),
            None => Err(e),
        },
    }?;
    metadata.mapped_variable = Some(mapped_variable.clone());

    let mut mapping_node = nodes[1].clone();

    match mapped_variable {
        Variable::Vector(vec) => Ok(Variable::Vector(
            vec.into_iter()
                .map(|v| {
                    mapping_node.nodes[0] = v.to_node();
                    mapping_node.start_evaluation(variables, subtrees).unwrap()
                })
                .collect(),
        )),
        Variable::Object(obj) => todo!(),
        _ => Err(Error::new_eval_internal(
            String::from("map_function"),
            format!(
                "only vector or object can be mapped, got: {:?}",
                mapped_variable
            ),
        )),
    }
}

fn filter(
    nodes: &[Node],
    variables: &mut HashMap<String, Variable>,
    subtrees: &HashMap<String, Vec<Node>>,
    metadata: &mut EvalMetadata,
) -> Result<Variable> {
    assert_eq!(nodes.len(), 2);

    let filtered_variable = nodes[0].start_evaluation(variables, subtrees)?;

    match filtered_variable {
        Variable::Vector(vec) => Ok(Variable::Vector(
            vec.iter()
                .map(|v| v.clone())
                .filter(|v| {
                    let mut filtering_node = nodes[1].clone();

                    filtering_node.change_special_function_placeholder(v.to_node());
                    // filtering_node.nodes[0] = v.to_node();
                    filtering_node
                        .start_evaluation(variables, subtrees)
                        .unwrap()
                        .is_true()
                })
                .collect(),
        )),
        _ => Err(Error::new_eval_internal(
            String::from("map_function"),
            format!(
                "only vector or object can be filtered, got: {:?}",
                filtered_variable
            ),
        )),
    }
}

/// Parses single Variable to given type.
fn parse_single_param<T>(nodes: &[Variable]) -> Result<T>
where
    T: std::str::FromStr + std::fmt::Debug,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    let param = nodes.first().ok_or_else(|| {
        Error::new_eval_internal(
            "parse_single_param".to_string(),
            "There's not variable".to_string(),
        )
    })?;
    parse_type(param)
}

/// Parses Variable to given type.
fn parse_type<T>(v: &Variable) -> Result<T>
where
    T: std::str::FromStr + std::fmt::Debug,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    if let Variable::String(s) = v {
        let a = s.parse::<T>().map_err(|_| {
            Error::new_eval_internal("parse_type".to_string(), "Could not parse".to_string())
        })?;
        return Ok(a);
    }
    Err(Error::new_eval_internal(
        "parse_type".to_string(),
        "param is not a Variable::String".to_string(),
    ))
}

/// All supported keyword that can be used in steps declarations.
#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Keyword {
    None,   // default value, not supported by language.
    Define, // defines new variable: DEFINE(var, 1).
    Get,    // returns defined variable: VAR(var).
    Json,   // returns Variable::Json: JSON("{}").
    Object, // return Variable::Object: Object(a, 1, b, INT(3), c, BOOL(true))
    Vec,    // returns Variable::Vec: VEC(1,2,3,4).
    Extract,
    Bool,
    Int,
    Float,
    Add,
    Sub,
    Div,
    Mult,
    HTTP,       // performs http request, has to return Variable::Json.
    Log,        // logs given Variable.
    RunSubtree, // takes 1 argument, subtree name: RunSubtree(subtree_name).
    // Takes no arguments, breaks from RunSubtree.
    // If RunSubtree are nested it'll break to root point.
    Break,
    If, // conditional run: IF(BOOL(true), RunSubtree(subtree_name)).
    // Returns true if two Variable are equal.
    // Can be chained like that: Eq(Eq(INT(1), INT(1)), Eq(FLOAT(2.5), FLOAT(2.5))).
    Eq,
    Neq,
    Map,    // can be used for vector/object values mapping: MAP(VEC(1,2,3), ADD(x, INT(4)))
    Filter, // can be used for vector/object values filtering: FILTER(VEC(1,2,3), EQ(X, 2)))
}

impl Keyword {
    fn from_string(s: &str) -> Option<Self> {
        let s = match s.to_lowercase().as_str() {
            "define" => Self::Define,
            "get" => Self::Get,
            "json" => Self::Json,
            "vec" => Self::Vec,
            "extract" => Self::Extract,
            "bool" => Self::Bool,
            "int" => Self::Int,
            "float" => Self::Float,
            "add" => Self::Add,
            "sub" => Self::Sub,
            "div" => Self::Div,
            "mult" => Self::Mult,
            "object" => Self::Object,
            "http" => Self::HTTP,
            "log" => Self::Log,
            "runsubtree" => Self::RunSubtree,
            "break" => Self::Break,
            "if" => Self::If,
            "eq" => Self::Eq,
            "neq" => Self::Neq,
            "map" => Self::Map,
            "filter" => Self::Filter,
            _ => Self::None,
        };
        if s == Self::None {
            return None;
        }
        Some(s)
    }

    /// Returns error if there's invalid number of arguments for given Keyword.
    fn check_arguments_count(&self, nodes: &[Variable]) -> Result<()> {
        let wanted = match self {
            Keyword::None | Keyword::Break => 0,
            Keyword::Get
            | Keyword::Json
            | Keyword::Object
            | Keyword::Bool
            | Keyword::Int
            | Keyword::Float
            | Keyword::HTTP
            | Keyword::Log
            | Keyword::RunSubtree => 1,
            Keyword::Define
            | Keyword::Add
            | Keyword::Sub
            | Keyword::Div
            | Keyword::Mult
            | Keyword::If
            | Keyword::Eq
            | Keyword::Neq
            | Keyword::Map
            | Keyword::Filter => 2,
            Keyword::Vec => {
                if nodes.len() == 0 {
                    return Err(Error::new_eval_internal(
                        String::from("Keyword::check_arguments_count"),
                        format!(
                            "keyword: {:?} - wanted at least 1 argument, got {}",
                            self,
                            nodes.len()
                        ),
                    ));
                }
                nodes.len()
            }
            Keyword::Extract => {
                if nodes.len() > 3 && nodes.len() < 2 {
                    return Err(Error::new_eval_internal(
                        String::from("Keyword::check_arguments_count"),
                        format!(
                            "keyword: {:?} - wanted 2 or 3 arguments, got {}",
                            self,
                            nodes.len()
                        ),
                    ));
                }
                nodes.len()
            }
        };
        nodes
            .len()
            .eq(&wanted)
            .then(|| 0)
            .ok_or(Error::new_eval_internal(
                String::from("Keyword::check_arguments_count"),
                format!(
                    "keyword: {:?} - wanted {} arguments, got {}",
                    self,
                    wanted,
                    nodes.len()
                ),
            ))
            .map(|_| ())
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Represents one independent piece of declaration.
pub enum Token {
    LeftBracket,
    RightBracket,
    Comma,
    Keyword(Keyword),
    Var(String),
}

/// Takes care of creating Tokens from wanted declaration.
pub struct Lexer<'a> {
    text: &'a str,
    pos: usize,
    current_char: char,
    done: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(text: &'a str) -> Self {
        Lexer {
            text,
            pos: 0,
            current_char: text.chars().next().unwrap(),
            done: false,
        }
    }

    fn advance(&mut self) {
        self.pos += 1;
        if self.pos < self.text.len() {
            self.current_char = self.text.chars().nth(self.pos).unwrap();
        } else {
            self.done = true
        }
    }

    fn skip(&self, ch: char) -> bool {
        ch.is_whitespace() || ch == '"' || ch == '{' || ch == '}' || ch == ':'
    }

    /// Create tokens from wanted declaration.
    pub fn make_tokens(&mut self) -> Vec<Token> {
        let mut tokens: Vec<Token> = vec![];

        while !self.done {
            if self.skip(self.current_char) {
                self.advance();
                continue;
            } else if self.current_char == '(' {
                tokens.push(Token::LeftBracket);
                self.advance()
            } else if self.current_char == ')' {
                tokens.push(Token::RightBracket);
                self.advance()
            } else if self.current_char == ',' {
                tokens.push(Token::Comma);
                self.advance()
            } else {
                tokens.push(self.make_word())
            }
        }
        tokens
    }

    fn make_word(&mut self) -> Token {
        let mut word = String::new();
        let mut apostrophe_found = false; // if so, we will have to find another end of whole string;
        let mut ending_apostrophe_missing = true;

        while !self.done {
            if self.current_char == '\'' {
                self.advance();
                if !apostrophe_found {
                    apostrophe_found = true;
                    continue;
                } else {
                    ending_apostrophe_missing = false;
                    break;
                }
            }
            if (self.current_char.is_alphanumeric() || self.current_char == '.') || apostrophe_found
            {
                word.push(self.current_char);
                self.advance();
            } else {
                break;
            }
        }
        if apostrophe_found && ending_apostrophe_missing {
            panic!("string with start but without the end")
        }

        if let Some(f) = Keyword::from_string(&word) {
            Token::Keyword(f)
        } else {
            Token::Var(word)
        }
    }
}

/// Takes vector of Tokens created by Lexer and parses them into Nodes tree.
pub struct Parser {
    tokens: Vec<Token>,
    token_inx: usize,
    current_token: Token,
    done: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let t = tokens.first().unwrap().clone();
        Parser {
            tokens,
            token_inx: 0,
            current_token: t,
            done: false,
        }
    }

    fn advance(&mut self) {
        self.token_inx += 1;
        if self.token_inx < self.tokens.len() {
            self.current_token = self.tokens.get(self.token_inx).unwrap().clone();
        } else {
            self.done = true
        }
    }

    pub fn parse(&mut self) -> Option<Node> {
        self.parse_v2()
    }

    /// Creates Nodes tree from given Tokens.
    fn parse_v2(&mut self) -> Option<Node> {
        if self.done {
            return None;
        }
        let mut pt: Node;
        if let Token::Keyword(s) = &self.current_token {
            pt = Node::new_keyword(s.clone());
        } else {
            panic!("first token is not keyword")
        }
        while !self.done {
            self.advance();
            match self.current_token {
                Token::RightBracket => {
                    break;
                }
                Token::Var(ref v) => pt.push(Node::new_var(v.clone())),
                Token::Keyword(_) => {
                    self.parse_v2().and_then(|parsed| Some(pt.push(parsed)));
                }
                _ => {}
            }
        }
        if pt.value == NodeEnum::None {
            None
        } else {
            Some(pt)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Lexer, Result};
    use crate::{
        error::types::Error,
        lang::{
            engine::{evaluate, fire, Definition, Engine, SubTree},
            eval::EvalForest,
            lexer::{Keyword, Node, NodeEnum, Parser, Token},
            variable::Variable,
        },
    };
    use serde_json::Value;
    use std::collections::HashMap;

    #[test]
    fn test_simple_lexer() {
        let mut lexer = Lexer::new("DEFINE)");
        let tokens = lexer.make_tokens();
        let wanted: Vec<Token> = vec![Token::Keyword(Keyword::Define), Token::RightBracket];
        assert_eq!(tokens, wanted);
    }

    #[test]
    fn test_lexer_with_apostrophes() {
        let map_str = r#"
        {
            "kid":"kidvalue",
            "kty":"RSA",
            "use":"sig",
            "n":"nvalue",
            "e":"evalue"
        }"#;
        let t = format!("OBJECT('{}')", map_str);
        let mut lexer = Lexer::new(&t);
        let tokens = lexer.make_tokens();
        let wanted: Vec<Token> = vec![
            Token::Keyword(Keyword::Object),
            Token::LeftBracket,
            Token::Var(map_str.to_string()),
            Token::RightBracket,
        ];
        assert_eq!(tokens, wanted);
    }

    #[test]
    fn test_lexer() {
        let mut lexer = Lexer::new("DEFINE(var, VEC(1,BOOL(2),3,FLOAT(4.0)))");

        let tokens = lexer.make_tokens();
        let wanted: Vec<Token> = vec![
            Token::Keyword(Keyword::Define),
            Token::LeftBracket,
            Token::Var(String::from("var")),
            Token::Comma,
            Token::Keyword(Keyword::Vec),
            Token::LeftBracket,
            Token::Var(String::from("1")),
            Token::Comma,
            Token::Keyword(Keyword::Bool),
            Token::LeftBracket,
            Token::Var(String::from("2")),
            Token::RightBracket,
            Token::Comma,
            Token::Var(String::from("3")),
            Token::Comma,
            Token::Keyword(Keyword::Float),
            Token::LeftBracket,
            Token::Var(String::from("4.0")),
            Token::RightBracket,
            Token::RightBracket,
            Token::RightBracket,
        ];
        assert_eq!(tokens, wanted);

        let mut parser = Parser::new(tokens);
        let got = parser.parse().unwrap();

        let mut main = Node::new_keyword(Keyword::Define);
        main.push(Node::new_var(String::from("var")));
        let mut vec = Node::new_keyword(Keyword::Vec);
        let v1 = Node::new_var(String::from("1"));
        let v2 = Node::new_keyword(Keyword::Bool).append(Node::new_var(String::from("2")));
        let v3 = Node::new_var(String::from("3"));
        let v4 = Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("4.0")));
        vec.push(v1);
        vec.push(v2);
        vec.push(v3);
        vec.push(v4);
        main.push(vec);
        assert_eq!(got, main);
    }

    #[test]
    fn test_lexer_v2() {
        let mut lexer = Lexer::new("DEFINE(var3, EXTRACT(var, use))");
        let tokens = lexer.make_tokens();

        let mut parser = Parser::new(tokens);
        let got = parser.parse().unwrap();

        let main = Node::new_keyword(Keyword::Define)
            .append(Node::new_var(String::from("var3")))
            .append(
                Node::new_keyword(Keyword::Extract)
                    .append(Node::new_var(String::from("var")))
                    .append(Node::new_var(String::from("use"))),
            );
        assert_eq!(got, main);
    }

    #[test]
    fn test_lexer_v3() {
        let tokens = Lexer::new("DEFINE(var3, qwdqw)").make_tokens();
        let got = Parser::new(tokens).parse().unwrap();

        let r = Node::new_keyword(Keyword::Define)
            .append(Node::new_var(String::from("var3")))
            .append(Node::new_var(String::from("qwdqw")));

        assert_eq!(got, r);
    }

    #[test]
    fn test_eval() {
        let mut state = HashMap::default();
        let subtrees = HashMap::new();

        let var = String::from("var");
        let n1 = Node::new_var(var.clone());
        assert_eq!(
            n1.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::String(var)
        );

        let n1 = Node::new_keyword(Keyword::Bool).append(Node::new_var(String::from("true")));
        assert_eq!(
            n1.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Bool(true)
        );

        let n1 = Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.3")));
        assert_eq!(
            n1.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Float(2.3)
        );

        let n2 = Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("123")));
        assert_eq!(
            n2.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Int(123)
        );

        let n3 = Node::new_keyword(Keyword::Add)
            .append(n1)
            .append(n2.clone());
        assert_eq!(
            n3.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Float(125.3)
        );

        let n4 = Node::new_keyword(Keyword::Add)
            .append(n2.clone())
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("200"))));
        assert_eq!(
            n4.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Int(323)
        );

        let n5 = Node::new_keyword(Keyword::Sub)
            .append(n2)
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("23"))));
        assert_eq!(
            n5.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Int(100)
        );

        let n5 = Node::new_keyword(Keyword::Div)
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("20"))))
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("2"))));
        assert_eq!(
            n5.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Int(10)
        );

        let n5 = Node::new_keyword(Keyword::Div)
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("20.0"))))
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"))));
        assert_eq!(
            n5.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Float(8.)
        );

        let n5 = Node::new_keyword(Keyword::Div)
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("-20"))))
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("2"))));
        assert_eq!(
            n5.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Int(-10)
        );

        let n5 = Node::new_keyword(Keyword::Div)
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("-20.0"))))
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"))));
        assert_eq!(
            n5.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Float(-8.)
        );

        let n5 = Node::new_keyword(Keyword::Mult)
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("-20.0"))))
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"))));
        assert_eq!(
            n5.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Float(-50.)
        );

        let n5 = Node::new_keyword(Keyword::Mult)
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("-20"))))
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("2"))));
        assert_eq!(
            n5.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Int(-40)
        );
    }

    #[test]
    fn test_eval_complex() {
        let mut state = HashMap::default();
        let subtrees = HashMap::new();

        let n1 = Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.3")));

        let n2 = Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("123")));
        let n3 = Node::new_keyword(Keyword::Add)
            .append(n1)
            .append(n2.clone());
        let n4 = Node::new_keyword(Keyword::Mult)
            .append(n3)
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"))));
        assert_eq!(
            n4.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Float(313.25)
        );
    }

    #[test]
    fn test_parse_eval() {
        let mut state = HashMap::default();
        let subtrees = HashMap::new();

        let mut lexer = Lexer::new("VEC(1,BOOL(true),3,FLOAT(4.0))");
        let tokens = lexer.make_tokens();
        let mut parser = Parser::new(tokens);
        let got = parser.parse().unwrap();
        println!("{:?}", got);
        assert_eq!(
            got.start_evaluation(&mut state, &subtrees).unwrap(),
            Variable::Vector(vec![
                Variable::String(String::from("1")),
                Variable::Bool(true),
                Variable::String(String::from("3")),
                Variable::Float(4.0)
            ])
        )
    }

    fn fire_for_test(
        def: Definition,
        state: &mut HashMap<String, Variable>,
        subtrees: &HashMap<String, Vec<Node>>,
    ) -> Result<()> {
        for step in def {
            let root = Parser::new(Lexer::new(&step).make_tokens())
                .parse()
                .unwrap();
            root.start_evaluation(state, subtrees)?;
        }
        Ok(())
    }
    /// Runs single test scenario.
    fn test(def: Definition, var_name: String, value: Variable) {
        let mut state = HashMap::default();
        let subtrees = HashMap::new();

        fire_for_test(def, &mut state, &subtrees).unwrap();
        assert_eq!(*state.get(&var_name).unwrap(), value);
    }

    #[test]
    fn test_parse_define() {
        let def = Definition::new(vec!["DEFINE(var, lol)".to_string()]);
        test(
            def,
            "var".to_string(),
            Variable::String(String::from("lol")),
        );
    }

    #[test]
    fn test_parse_int_define() {
        let def = Definition::new(vec!["DEFINE(var, int(2))".to_string()]);
        test(def, "var".to_string(), Variable::Int(2));
    }

    #[test]
    fn test_parse_float_define() {
        let def = Definition::new(vec!["DEFINE(var, float(2))".to_string()]);
        test(def, "var".to_string(), Variable::Float(2.));
    }

    #[test]
    fn test_parse_object_define() {
        let map_str = r#"
        {
            "kid":"kidvalue",
            "kty":"RSA",
            "use":"sig",
            "n":"nvalue",
            "e":"evalue"
        }"#;

        let mut map: HashMap<String, Variable> = HashMap::new();
        let subtrees = HashMap::new();

        map.insert(
            String::from("kid"),
            Variable::String(String::from("kidvalue")),
        );
        map.insert(String::from("kty"), Variable::String(String::from("RSA")));
        map.insert(String::from("use"), Variable::String(String::from("sig")));
        map.insert(String::from("n"), Variable::String(String::from("nvalue")));
        map.insert(String::from("e"), Variable::String(String::from("evalue")));

        let mut state = HashMap::default();
        let def = Definition::new(vec![
            format!("DEFINE(var, OBJECT('{}'))", map_str).to_string(),
            String::from("DEFINE(var2, EXTRACT(GET(var), kty))"),
            String::from("DEFINE(var3, EXTRACT(GET(var), use))"),
            String::from("DEFINE(var4, EXTRACT(GET(var), n))"),
        ]);
        fire_for_test(def, &mut state, &subtrees).unwrap();
        assert_eq!(*state.get("var").unwrap(), Variable::Object(map));
        assert_eq!(
            *state.get("var2").unwrap(),
            Variable::String(String::from("RSA"))
        );
        assert_eq!(
            *state.get("var3").unwrap(),
            Variable::String(String::from("sig"))
        );
        assert_eq!(
            *state.get("var4").unwrap(),
            Variable::String(String::from("nvalue"))
        );
    }

    #[test]
    fn test_parse_object_embedded_define() {
        let map_str = r#"
        {
            "kid":"kid-value",
            "kty": {
                "use":"sig",
                "n":"n-value",
                "e":"e-value"
            }
        }"#;

        let subtrees = HashMap::new();
        let mut embedded: HashMap<String, Variable> = HashMap::new();
        embedded.insert(String::from("use"), Variable::String(String::from("sig")));
        embedded.insert(String::from("n"), Variable::String(String::from("n-value")));
        embedded.insert(String::from("e"), Variable::String(String::from("e-value")));
        let mut map: HashMap<String, Variable> = HashMap::new();
        map.insert(
            String::from("kid"),
            Variable::String(String::from("kid-value")),
        );
        let obj = Variable::Object(embedded);
        map.insert(String::from("kty"), obj.clone());
        let mut state = HashMap::default();
        let def = Definition::new(vec![
            format!("DEFINE(var, object('{}'))", map_str).to_string(),
            String::from("DEFINE(var2, EXTRACT(GET(var), kty))"),
        ]);
        fire_for_test(def, &mut state, &subtrees).unwrap();
        assert_eq!(*state.get("var").unwrap(), Variable::Object(map));
        assert_eq!(*state.get("var2").unwrap(), obj);
    }

    #[test]
    fn test_parse_json_define() {
        let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#;
        // Parse the string of data into serde_json::Value.
        let v: Value = serde_json::from_str(data).unwrap();

        let mut state = HashMap::default();
        let subtrees = HashMap::new();

        let def = Definition::new(vec![
            format!("DEFINE(var, JSON('{}'))", data).to_string(),
            "DEFINE(var2, EXTRACT(GET(var), name))".to_string(),
        ]);
        fire_for_test(def, &mut state, &subtrees).unwrap();
        assert_eq!(*state.get("var").unwrap(), Variable::Json(v));
        assert_eq!(
            *state.get("var2").unwrap(),
            Variable::String(String::from("John Doe"))
        );
    }

    #[test]
    fn test_parse_array_define() {
        let def = Definition::new(vec!["DEFINE(var, VEC(1,2,3,4))".to_string()]);
        test(
            def,
            "var".to_string(),
            Variable::Vector(vec![
                Variable::String(String::from("1")),
                Variable::String(String::from("2")),
                Variable::String(String::from("3")),
                Variable::String(String::from("4")),
            ]),
        );
    }

    #[test]
    fn test_parse_array_types_define() {
        let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#;
        // Parse the string of data into serde_json::Value.
        let v: Value = serde_json::from_str(data).unwrap();
        let def = Definition::new(vec![format!(
            "DEFINE(var, VEC(1, INT(2), FLOAT(3.2), JSON('{}')))",
            data
        )
        .to_string()]);
        test(
            def,
            "var".to_string(),
            Variable::Vector(vec![
                Variable::String(String::from("1")),
                Variable::Int(2),
                Variable::Float(3.2),
                Variable::Json(v),
            ]),
        );
    }

    #[test]
    fn test_parse_array_define_extract() {
        let def = Definition::new(vec![
            String::from("DEFINE(var, VEC(1,2,3,4))"),
            String::from("DEFINE(var2, EXTRACT(GET(var), 3))"),
        ]);
        test(def, "var2".to_string(), Variable::String(String::from("4")));
    }

    #[test]
    fn test_parse_array_define_extract_deep() {
        let def = Definition::new(vec![
            String::from("DEFINE(var, VEC(1,VEC(1,VEC(1, VEC(1, 2)))))"),
            String::from("DEFINE(var2, EXTRACT(GET(var), 1, BOOL(true)))"),
        ]);
        test(def, "var2".to_string(), Variable::String(String::from("2")));
    }

    #[test]
    fn test_parse_get_define() {
        let data = r#"{
            "userId": 1,
            "id": 1,
            "title": "delectus aut autem",
            "completed": false
        }"#;
        let v: Value = serde_json::from_str(data).unwrap();

        let def = Definition::new(vec![String::from(
            "DEFINE(var, HTTP('https://jsonplaceholder.typicode.com/todos/1'))",
        )]);
        test(def, "var".to_string(), Variable::Json(v));
    }

    #[test]
    fn test_parse_get_define_extract() {
        let def = Definition::new(vec![
            String::from("DEFINE(var, HTTP('https://jsonplaceholder.typicode.com/todos/1'))"),
            String::from("DEFINE(var2, EXTRACT(GET(var), title))"),
        ]);
        test(
            def,
            "var2".to_string(),
            Variable::String(String::from("delectus aut autem")),
        );
    }

    #[test]
    fn test_tree_serialization() {
        let data = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#;

        let tokens = Lexer::new(
            format!("DEFINE(var, VEC(1, INT(2), FLOAT(3.2), JSON('{}')))", data).as_str(),
        )
        .make_tokens();
        let root = Parser::new(tokens).parse().unwrap();

        let serialized = serde_json::to_string(&root).unwrap();
        let deserialized = serde_json::from_str::<Node>(&serialized).unwrap();
        assert_eq!(root, deserialized);
    }

    #[test]
    fn test_run_subtree() {
        let definition = Definition {
            steps: vec![
                String::from("DEFINE(IN, INT(20))"),
                String::from("RunSubtree(testsubtree)"),
            ],
            subtrees: Some(vec![SubTree {
                name: String::from("testsubtree"),
                input_type: None,
                definition: Definition {
                    steps: vec![String::from("DEFINE(OUT, ADD(GET(IN), INT(10)))")],
                    subtrees: None,
                },
            }]),
        };
        let eval_forest = EvalForest::from_definition(&definition);
        let out = evaluate(None, &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(30));

        let definition = Definition {
            steps: vec![String::from("RunSubtree(testsubtree)")],
            subtrees: Some(vec![
                SubTree {
                    name: String::from("testsubtree"),
                    input_type: None,
                    definition: Definition {
                        steps: vec![String::from("RunSubtree(testsubtree2)")],
                        subtrees: None,
                    },
                },
                SubTree {
                    name: String::from("testsubtree2"),
                    input_type: None,
                    definition: Definition {
                        steps: vec![String::from("DEFINE(OUT, INT(400))")],
                        subtrees: None,
                    },
                },
            ]),
        };
        let eval_forest = EvalForest::from_definition(&definition);
        let out = evaluate(None, &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(400));
    }

    // #[test]
    // fn test_run_subtree_for_each() {
    //     // VECTOR TEST.
    //     let definition = Definition {
    //         steps: vec![String::from(
    //             "DEFINE(OUT, RunSubtreeForEach(testsubtree, VEC(GET(IN), INT(1), INT(2), INT(3))))",
    //         )],
    //         subtrees: Some(vec![SubTree {
    //             name: String::from("testsubtree"),
    //             input_type: None,
    //             definition: Definition {
    //                 steps: vec![String::from("DEFINE(OUT, ADD(GET(IN), INT(10)))")],
    //                 subtrees: None,
    //             },
    //         }]),
    //     };

    //     let eval_forest = EvalForest::from_definition(&definition);
    //     let mut engine = Engine::new(Variable::Int(10), eval_forest);
    //     engine.fire().expect("fire failed");
    //     assert_eq!(
    //         engine.get("OUT").expect("there's not OUT variable"),
    //         &Variable::Vector(vec![
    //             Variable::Int(20),
    //             Variable::Int(11),
    //             Variable::Int(12),
    //             Variable::Int(13)
    //         ])
    //     );

    //     // OBJECT TEST.
    //     let map_str = r#"
    //     {
    //         "kid": 1,
    //         "kty": 2,
    //         "use": 3,
    //         "n": 4,
    //         "e": 5
    //     }"#;
    //     let t = format!("OBJECT('{}')", map_str);

    //     let definition = Definition {
    //         steps: vec![
    //             format!("DEFINE(var, RunSubtreeForEach(testsubtree, {}))", t,),
    //             String::from("DEFINE(OUT, RunSubtreeForEach(testsubtree2, GET(var)))"),
    //         ],
    //         subtrees: Some(vec![
    //             SubTree {
    //                 name: String::from("testsubtree"),
    //                 input_type: None,
    //                 definition: Definition {
    //                     steps: vec![String::from("DEFINE(OUT, ADD(GET(IN), INT(10)))")],
    //                     subtrees: None,
    //                 },
    //             },
    //             SubTree {
    //                 name: String::from("testsubtree2"),
    //                 input_type: None,
    //                 definition: Definition {
    //                     steps: vec![String::from("DEFINE(OUT, MULT(GET(IN), INT(10)))")],
    //                     subtrees: None,
    //                 },
    //             },
    //         ]),
    //     };

    //     let eval_forest = EvalForest::from_definition(&definition);
    //     let mut engine = Engine::new(Variable::Int(10), eval_forest);
    //     engine.fire().expect("fire failed");
    //     assert_eq!(
    //         engine.get("OUT").expect("there's not OUT variable"),
    //         &Variable::Object(HashMap::from([
    //             (String::from("kid"), Variable::Int(110)),
    //             (String::from("kty"), Variable::Int(120)),
    //             (String::from("use"), Variable::Int(130)),
    //             (String::from("n"), Variable::Int(140)),
    //             (String::from("e"), Variable::Int(150)),
    //         ]))
    //     )
    // }

    // #[test]
    // fn test_run_subtree_for_each_invalid_type() {
    //     let definition = Definition {
    //         steps: vec![String::from(
    //             "DEFINE(OUT, RunSubtreeForEach(testsubtree, VEC(GET(IN), INT(1), INT(2), INT(3))))",
    //         )],
    //         subtrees: Some(vec![SubTree {
    //             name: String::from("testsubtree"),
    //             input_type: None,
    //             definition: Definition {
    //                 steps: vec![String::from("DEFINE(OUT, FLOAT(1.0))")],
    //                 subtrees: None,
    //             },
    //         }]),
    //     };

    //     let eval_forest = EvalForest::from_definition(&definition);
    //     let mut engine = Engine::new(Variable::Int(10), eval_forest);
    //     let result = engine.fire();
    //     assert!(result.is_err());
    //     assert_eq!(
    //         result.err().unwrap(),
    //         Error::new_eval_internal(
    //             String::from("run_subtree_for_each"),
    //             String::from("invalid type of OUT variable: Float(1.0)"),
    //         )
    //     );
    // }

    #[test]
    fn test_if_function() {
        env_logger::try_init();

        let def = Definition::new(vec![String::from("DEFINE(var, IF(BOOL(true), INT(1)))")]);
        test(def, String::from("var"), Variable::Int(1));

        let def = Definition::new(vec![String::from("DEFINE(var, IF(BOOL(false), INT(1)))")]);
        test(def, String::from("var"), Variable::None);

        let def = Definition::new(vec![
            String::from("DEFINE(IN, VEC(1,2,3,4))"),
            String::from("DEFINE(var, IF(BOOL(true), GET(IN)))"),
        ]);
        test(
            def,
            String::from("var"),
            Variable::Vector(vec![
                Variable::String(String::from("1")),
                Variable::String(String::from("2")),
                Variable::String(String::from("3")),
                Variable::String(String::from("4")),
            ]),
        );
    }

    #[test]
    fn test_if_function_with_run_subtree() {
        let definition = Definition {
            steps: vec![String::from("IF(INT(1), RunSubtree(testsubtree))")],
            subtrees: Some(vec![SubTree {
                name: String::from("testsubtree"),
                input_type: None,
                definition: Definition {
                    steps: vec![String::from("DEFINE(OUT, ADD(GET(IN), INT(10)))")],
                    subtrees: None,
                },
            }]),
        };

        let eval_forest = EvalForest::from_definition(&definition);
        let out = evaluate(Some(Variable::Int(125)), &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(135));

        let definition = Definition {
            steps: vec![String::from("IF(INT(2), RunSubtree(testsubtree))")],
            subtrees: Some(vec![SubTree {
                name: String::from("testsubtree"),
                input_type: None,
                definition: Definition {
                    steps: vec![String::from("DEFINE(OUT, ADD(GET(IN), INT(10)))")],
                    subtrees: None,
                },
            }]),
        };

        let eval_forest = EvalForest::from_definition(&definition);
        let out = evaluate(Some(Variable::Int(125)), &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(125));

        let definition = Definition {
            steps: vec![String::from("RunSubtree(testsubtree)")],
            subtrees: Some(vec![
                SubTree {
                    name: String::from("testsubtree"),
                    input_type: None,
                    definition: Definition {
                        steps: vec![String::from("IF(GET(IN), RunSubtree(testsubtree2))")],
                        subtrees: None,
                    },
                },
                SubTree {
                    name: String::from("testsubtree2"),
                    input_type: None,
                    definition: Definition {
                        steps: vec![String::from("DEFINE(OUT, INT(155))")],
                        subtrees: None,
                    },
                },
            ]),
        };

        let eval_forest = EvalForest::from_definition(&definition);
        let out = evaluate(Some(Variable::Int(1)), &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(155));

        let out = evaluate(Some(Variable::Int(2)), &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(2));
    }

    #[test]
    fn test_if_function_without_define() {
        let definition = Definition {
            steps: vec![String::from("IF(INT(1), DEFINE(var, INT(100)))")],
            subtrees: None,
        };
        let eval_forest = EvalForest::from_definition(&definition);
        let mut engine = Engine::new(Variable::Int(10), eval_forest);
        engine.fire().expect("fire failed");
        assert_eq!(engine.get("var").unwrap(), &Variable::Int(100));

        let definition = Definition {
            steps: vec![String::from("IF(INT(123), DEFINE(var, INT(100)))")],
            subtrees: None,
        };
        let eval_forest = EvalForest::from_definition(&definition);
        let mut engine = Engine::new(Variable::Int(10), eval_forest);
        engine.fire().expect("fire failed");
        assert!(engine.get("var").is_none());

        let definition = Definition {
            steps: vec![String::from("IF(BOOL(true), DEFINE(var, INT(100)))")],
            subtrees: None,
        };
        let eval_forest = EvalForest::from_definition(&definition);
        let mut engine = Engine::new(Variable::Int(10), eval_forest);
        engine.fire().expect("fire failed");
        assert_eq!(engine.get("var").unwrap(), &Variable::Int(100));
    }

    #[test]
    fn test_break_parse() {
        let root = Parser::new(Lexer::new("BREAK").make_tokens())
            .parse()
            .unwrap();
        assert_eq!(root.value, NodeEnum::Keyword(Keyword::Break));
        assert_eq!(root.nodes.len(), 0);
    }

    #[test]
    fn test_break_keyword() {
        env_logger::try_init();
        let definition = Definition {
            steps: vec![String::from("RunSubtree(testsubtree)")],
            subtrees: Some(vec![
                SubTree {
                    name: String::from("testsubtree"),
                    input_type: None,
                    definition: Definition {
                        steps: vec![
                            String::from("BREAK"),
                            String::from("RunSubtree(testsubtree2)"), // should not be called.
                        ],
                        subtrees: None,
                    },
                },
                SubTree {
                    name: String::from("testsubtree2"),
                    input_type: None,
                    definition: Definition {
                        steps: vec![String::from("DEFINE(OUT, INT(155))")], // should not be called.
                        subtrees: None,
                    },
                },
            ]),
        };

        let eval_forest = EvalForest::from_definition(&definition);
        let out = evaluate(Some(Variable::Int(2)), &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(2));

        let definition = Definition {
            steps: vec![String::from("RunSubtree(testsubtree)")],
            subtrees: Some(vec![SubTree {
                name: String::from("testsubtree"),
                input_type: None,
                definition: Definition {
                    steps: vec![
                        String::from("DEFINE(IN, SUB(GET(IN), INT(1)))"),
                        String::from("IF(GET(IN), DEFINE(OUT, GET(IN)))"),
                        String::from("IF(GET(IN), BREAK)"),
                        String::from("RunSubtree(testsubtree)"),
                    ],
                    subtrees: None,
                },
            }]),
        };

        let eval_forest = EvalForest::from_definition(&definition);
        let out = evaluate(Some(Variable::Int(5)), &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(1));

        let definition = Definition {
            steps: vec![String::from("RunSubtree(testsubtree)")],
            subtrees: Some(vec![SubTree {
                name: String::from("testsubtree"),
                input_type: None,
                definition: Definition {
                    steps: vec![
                        String::from("DEFINE(IN, SUB(GET(IN), INT(1)))"),
                        String::from("RunSubtree(testsubtree)"),
                    ],
                    subtrees: None,
                },
            }]),
        };

        let eval_forest = EvalForest::from_definition(&definition);
        let out = evaluate(Some(Variable::Int(5)), &eval_forest)
            .expect_err("should be stack overflow error");
        assert_eq!(
            out,
            Error::new_eval_internal(String::from("run_subtree"), String::from("stack overflow"))
        );
    }

    #[test]
    fn eq_test() {
        let def = Definition::new(vec![String::from(
            "DEFINE(OUT, Eq(Eq(INT(1), INT(1)), Eq(FLOAT(2.5), FLOAT(2.5))))",
        )]);
        test(def, String::from("OUT"), Variable::Bool(true));
    }

    #[test]
    fn map_test() {
        let def = Definition::new(vec![String::from(
            "DEFINE(OUT, MAP(VEC(INT(1), INT(2), INT(3)), ADD(X, INT(2))))",
        )]);
        test(
            def,
            String::from("OUT"),
            Variable::Vector(vec![Variable::Int(3), Variable::Int(4), Variable::Int(5)]),
        );

        let def = Definition::new(vec![
            String::from("DEFINE(IN, VEC(INT(11), INT(12), INT(13)))"),
            String::from("DEFINE(OUT, MAP(GET(IN), SUB(X, INT(2))))"),
        ]);
        test(
            def,
            String::from("OUT"),
            Variable::Vector(vec![Variable::Int(9), Variable::Int(10), Variable::Int(11)]),
        );

        let def = Definition::new(vec![
            String::from("DEFINE(IN, VEC(VEC(INT(10), INT(11)), VEC(INT(20), INT(40))))"),
            String::from("DEFINE(OUT, MAP(GET(IN), MAP(X, SUB(X, INT(2)))))"),
        ]);
        test(
            def,
            String::from("OUT"),
            Variable::Vector(vec![
                Variable::Vector(vec![Variable::Int(8), Variable::Int(9)]),
                Variable::Vector(vec![Variable::Int(18), Variable::Int(38)]),
            ]),
        );
    }

    #[test]
    fn filter_test() {
        let def = Definition::new(vec![String::from(
            "DEFINE(OUT, FILTER(VEC(INT(1), INT(2), INT(3)), EQ(X, INT(2))))",
        )]);
        test(
            def,
            String::from("OUT"),
            Variable::Vector(vec![Variable::Int(2)]),
        );

        let def = Definition::new(vec![String::from(
            "DEFINE(OUT, FILTER(VEC(BOOL(true), BOOL(false), BOOL(true)), EQ(X, BOOL(true))))",
        )]);
        test(
            def,
            String::from("OUT"),
            Variable::Vector(vec![Variable::Bool(true), Variable::Bool(true)]),
        );

        let def = Definition::new(vec![String::from(
            "DEFINE(OUT, FILTER(VEC(INT(1), INT(2), INT(3)), EQ(ADD(X, INT(2)), INT(4))))",
        )]);
        test(
            def,
            String::from("OUT"),
            Variable::Vector(vec![Variable::Int(2)]),
        );
    }
}
