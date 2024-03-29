use super::lexer::Keyword;
use super::variable::Variable;
use crate::error::types::{Error, Result};
use crate::lang::variable::value_object_to_variable_object;
use anyhow::{bail, Context};
use core::panic;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::Read;
use std::rc::Rc;
use std::{
    collections::HashMap,
    fmt::{self, Display},
};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
/// Enum for Node type.
pub enum NodeEnum {
    None,
    Keyword(Keyword),                     // Keyword is a supported function.
    Var { value: String, l_value: bool }, // Variable name or "default" evaluation of variable which is String.
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
        if self.stack.is_empty() {
            self.should_break = false;
        }
    }
}

#[derive(Default)]
pub struct SharedState {
    /// Holds variables state.
    pub variables: HashMap<String, Variable>,

    /// Holds parsed subtrees.
    pub subtress: HashMap<String, Vec<Node>>,

    /// Holds created readers for wanted mounted options.
    pub mounted: HashMap<String, Rc<RefCell<dyn Read>>>,

    pub eval_metadata: EvalMetadata,

    /// Holds variables during lifetime of started transaction.
    ///
    /// After transaction commit this variables will overwrite `variables`.
    /// After transaction rollback this variables will be discarded.
    transactions_variables: Option<HashMap<String, Variable>>,
}

impl SharedState {
    pub fn new(variables: HashMap<String, Variable>, subtress: HashMap<String, Vec<Node>>) -> Self {
        Self {
            variables,
            subtress,
            ..Default::default()
        }
    }

    pub fn new_with_mounted(
        variables: HashMap<String, Variable>,
        subtress: HashMap<String, Vec<Node>>,
        mounted: HashMap<String, Rc<RefCell<dyn Read>>>,
    ) -> Self {
        Self {
            variables,
            subtress,
            mounted,
            eval_metadata: EvalMetadata::default(),
            transactions_variables: None,
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
    pub fn new_keyword(keyword: Keyword) -> Self {
        Node {
            value: NodeEnum::Keyword(keyword),
            nodes: vec![],
        }
    }

    /// Creates new Node with NodeEnum::Var type.
    pub fn new_var(value: String, l_value: bool) -> Self {
        Node {
            value: NodeEnum::Var { value, l_value },
            nodes: vec![],
        }
    }

    /// Adds nodes to node.
    pub fn push(&mut self, pt: Node) {
        self.nodes.push(pt)
    }

    /// Appends node to nodes and return Self.
    pub fn append(&mut self, n: Node) -> Self {
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
        match &self.value {
            NodeEnum::Var { value, l_value: _ } if value == &String::from("X") => {
                *self = replacement.clone()
            }
            _ => (),
        }

        for i in 0..self.nodes.len() {
            self.nodes[i].change_special_function_placeholder(replacement.clone())
        }
    }

    pub fn start_evaluation(&self, state: &mut SharedState) -> Result<Variable> {
        let mut subtree_stack = Stack::default();
        self.eval(state, &mut subtree_stack)
    }

    /// Evaluates whole tree to a single Variable.
    /// Function can be chained with each other as shown below:
    ///   "VEC(1,BOOL(true),3,FLOAT(4.0))"
    /// this function will go trough tree created from that declaration
    /// and evaluate root node and all of nodes below in order to return
    /// single Variable as a result.
    pub fn eval(&self, state: &mut SharedState, stack: &mut Stack) -> Result<Variable> {
        match self.value {
            NodeEnum::None => Ok(Variable::None),
            NodeEnum::Keyword(ref keyword) => {
                if stack.should_break {
                    stack.pop(); // pop latest subtree from stack.
                    return Ok(Variable::None);
                }

                // must be checked before further evaluation.
                if let Keyword::If = keyword {
                    if !if_check(&self.nodes, state)? {
                        return Ok(Variable::None);
                    }
                }

                match keyword {
                    Keyword::If => {
                        if !if_check(&self.nodes, state)? {
                            return Ok(Variable::None);
                        }
                    }
                    Keyword::Map => {
                        return map_function(&self.nodes, state);
                    }
                    Keyword::Filter => return filter(&self.nodes, state),
                    _ => {}
                }

                let nodes = self
                    .nodes
                    .iter()
                    .map(|n| n.eval(state, stack))
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
                    Keyword::Define => define(&nodes, &mut state.variables),
                    Keyword::Get => get(&nodes, &state.variables),
                    Keyword::Json => json(&nodes),
                    Keyword::Object => object(&nodes),
                    Keyword::HTTP => http(&nodes),
                    Keyword::Log => log(&nodes),
                    Keyword::RunSubtree => run_subtree(&nodes, state, stack),
                    Keyword::If => if_return(&nodes),
                    Keyword::Eq => eq(&nodes),
                    Keyword::Neq => neq(&nodes),
                    Keyword::Break => {
                        break_function(stack);
                        Ok(Variable::None)
                    }
                    Keyword::ReadMountedToString => read_mounted_to_string(&nodes, state),
                    Keyword::Append => append(&nodes, state),
                    Keyword::Begin => begin(state),
                    Keyword::Commit => commit(state),
                    Keyword::Rollback => rollback(state),
                    _ => panic!("should not be reached"),
                }
            }
            NodeEnum::Var { ref value, l_value } => {
                // check if variable with given name exists.
                if !l_value {
                    if let Some(value) = state.variables.get(value) {
                        return Ok(value.clone());
                    }
                }

                Ok(Variable::String(value.clone()))
            }
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
    let deep = iter.next().map(|f| f.is_true()).unwrap_or_default(); // optional argument

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
        .map_err(|err| Error::new_eval_internal(String::from("get"), err.to_string()))?;

    let g = state.get(&v).ok_or_else(|| {
        Error::new_eval_internal(String::from("get"), format!("variable: {} not found", v))
    })?;
    Ok(g.clone())
}

// Returns Variable::Object parsed from json-like string.
fn object(nodes: &[Variable]) -> Result<Variable> {
    let v = parse_single_param::<String>(nodes)
        .map_err(|err| Error::new_eval_internal(String::from("object"), err.to_string()))?;

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
fn run_subtree(nodes: &[Variable], state: &mut SharedState, stack: &mut Stack) -> Result<Variable> {
    if stack.stack.len() == MAX_SUBTREE_STACK {
        return Err(Error::new_eval_internal(
            String::from("run_subtree"),
            String::from("stack overflow"),
        ));
    }

    let mut iter = nodes.iter();
    let subtree_name = iter.next().unwrap().to_str()?;

    debug!("run_subtree - running {} subtree", subtree_name);

    let subtree = state
        .subtress
        .get(subtree_name)
        .context("no subtree found")?
        .clone();

    stack.push(subtree_name.to_string()); // add subtree call to
    fire_subtree(&subtree, state, stack)?;

    Ok(Variable::None)
}

pub fn fire_subtree(roots: &[Node], state: &mut SharedState, stack: &mut Stack) -> Result<()> {
    for root in roots {
        root.eval(state, stack)?;
    }
    Ok(())
}

/// Function checks *first* element of nodes if can be evaluated to 'true'.
/// If so, second arguments as some operation will be run.
fn if_check(nodes: &[Node], state: &mut SharedState) -> Result<bool> {
    debug!("if_function - nodes: {:?}", nodes);
    // eval first node which is conditional value.
    nodes[0].start_evaluation(state).map(|v| v.is_true())
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

fn map_function(nodes: &[Node], state: &mut SharedState) -> Result<Variable> {
    assert_eq!(nodes.len(), 2);
    let mapped_variable = match nodes[0].start_evaluation(state) {
        Ok(v) => Ok(v),
        Err(e) => match &state.eval_metadata.mapped_variable {
            Some(v) => Ok(v.clone()),
            None => Err(e),
        },
    }?;
    state.eval_metadata.mapped_variable = Some(mapped_variable.clone());

    let mut mapping_node = nodes[1].clone();

    match mapped_variable {
        Variable::Vector(vec) => Ok(Variable::Vector(
            vec.into_iter()
                .map(|v| {
                    mapping_node.nodes[0] = v.to_node();
                    mapping_node.start_evaluation(state).unwrap()
                })
                .collect(),
        )),
        Variable::Object(_) => todo!(),
        _ => Err(Error::new_eval_internal(
            String::from("map_function"),
            format!(
                "only vector or object can be mapped, got: {:?}",
                mapped_variable
            ),
        )),
    }
}

fn filter(nodes: &[Node], state: &mut SharedState) -> Result<Variable> {
    assert_eq!(nodes.len(), 2);

    let filtered_variable = nodes[0].start_evaluation(state)?;

    match filtered_variable {
        Variable::Vector(vec) => Ok(Variable::Vector(
            vec.iter()
                .cloned()
                .filter(|v| {
                    let mut filtering_node = nodes[1].clone();

                    filtering_node.change_special_function_placeholder(v.to_node());
                    // filtering_node.nodes[0] = v.to_node();
                    filtering_node.start_evaluation(state).unwrap().is_true()
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

fn read_mounted_to_string(nodes: &[Variable], state: &mut SharedState) -> Result<Variable> {
    assert_eq!(nodes.len(), 1);

    let alias = parse_single_param::<String>(nodes).map_err(|err| {
        Error::new_eval_internal(String::from("read_mounted_to_string"), err.to_string())
    })?;

    let reader = state
        .mounted
        .get(&alias)
        .context("no reader for wanted alias")?;

    let mut buf = String::new();
    reader
        .borrow_mut()
        .read_to_string(&mut buf)
        .context("could not from buffer to string")?;

    Ok(Variable::String(buf))
}

/// Appends second argument(data) to first(appended variable) - which is initialized as varaible
/// and exists in SharedState. Function performs needed checks to maintain consitancty.
fn append(nodes: &[Variable], state: &mut SharedState) -> Result<Variable> {
    assert_eq!(nodes.len(), 2);

    let mut iter = nodes.iter();
    let appended = iter.next().unwrap().to_owned();
    let data = iter.next().unwrap().to_owned();

    if let Variable::String(appended) = appended {
        let initialized = state
            .variables
            .get_mut(&appended)
            .context("appended variable not initialized")?;

        match initialized {
            Variable::String(s) => {
                if let Variable::String(data) = data {
                    s.push_str(&data)
                } else {
                    // only string can be appended to a string.
                    return Err(Error::new_eval_invalid_type(
                        "append",
                        format!("{:?}", data).as_str(),
                        "Variable::String",
                    ));
                }
            }
            Variable::Vector(vector) => {
                // we can append anything to a vector.
                vector.push(data)
            }
            _ => {
                return Err(Error::new_eval_invalid_type(
                    "append",
                    format!("{:?}", initialized).as_str(),
                    "Variable::String | Variable::Vector",
                ));
            }
        }
    } else {
        return Err(Error::new_eval_invalid_type(
            "append",
            format!("{:?}", appended).as_str(),
            "Variable::String",
        ));
    }

    Ok(Variable::None)
}

/// Checks if transaction can be started, if so starts one.
fn begin(state: &mut SharedState) -> Result<Variable> {
    if let Some(_tx_variables) = &state.transactions_variables {
        return Err(Error::new_eval_internal(
            "begin",
            "transaction already started",
        ));
    }

    state.transactions_variables = Some(state.variables.clone());

    Ok(Variable::None)
}

/// Commits current transaction.
fn commit(state: &mut SharedState) -> Result<Variable> {
    if let Some(_tx_variables) = &state.transactions_variables {
        state.transactions_variables = None;

        return Ok(Variable::None);
    }

    Err(Error::new_eval_internal(
        "commit",
        "transaction wasn't started",
    ))
}

/// Commits current transaction.
fn rollback(state: &mut SharedState) -> Result<Variable> {
    if let Some(tx_variables) = &state.transactions_variables {
        state.variables = tx_variables.clone();
        state.transactions_variables = None;

        return Ok(Variable::None);
    }

    Err(Error::new_eval_internal(
        "rollback",
        "transaction wasn't started",
    ))
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
