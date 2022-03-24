use super::{engine::Engine, variable::Variable};
use crate::lang::variable::value_object_to_variable_object;
use core::panic;
use serde_json::Value;
use std::fmt::{self, Display};

#[derive(Debug, PartialEq, Clone)]
/// Enum for Node type.
enum NodeEnum {
    None,
    Keyword(Keyword), // Keyword is a supported function.
    Var(String),      // Variable name or "default" evaluation of variable which is String.
}

impl NodeEnum {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, PartialEq, Clone)]
/// Node represents single node in lexer chain.
/// Struct contains value which is type of Node -> var or keyword.
/// Vector of nodes are all params that were passed to keyword function and will
/// be evaluated during Node evaluation.
pub struct Node {
    value: NodeEnum,
    nodes: Vec<Box<Node>>,
}

impl Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

impl Node {
    /// Returns default value for Node.
    fn default() -> Self {
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
        self.nodes.push(Box::new(pt))
    }

    /// Appends node to nodes and return Self.
    fn append(&mut self, n: Node) -> Self {
        self.push(n);
        self.clone()
    }

    /// Evaluates whole tree to a single Variable.
    /// Function can be chained with each other as shown below:
    ///   "VEC(1,BOOL(true),3,FLOAT(4.0))"
    /// this function will go trough tree created from that declaration
    /// and evaluate root node and all of nodes below in order to return
    /// single Variable as a result.
    pub fn eval(&self, state: &mut Engine) -> Variable {
        match self.value {
            NodeEnum::None => todo!(),
            NodeEnum::Keyword(ref keyword) => {
                let nodes: Vec<Variable> = self.nodes.iter().map(|n| n.eval(state)).collect();
                match keyword {
                    Keyword::Bool => bool(&nodes),
                    Keyword::Int => int(&nodes),
                    Keyword::Float => float(&nodes),
                    Keyword::Add => add(&nodes),
                    Keyword::Min => sub(&nodes),
                    Keyword::Div => div(&nodes),
                    Keyword::Mult => mult(&nodes),
                    Keyword::Vec => Variable::Vector(nodes),
                    Keyword::Extract => extract(&nodes),
                    Keyword::Define => define(&nodes, state),
                    Keyword::Get => get(&nodes, state),
                    Keyword::Json => json(&nodes),
                    Keyword::Object => object(&nodes),
                    Keyword::HTTP => http(&nodes),
                    Keyword::None => panic!("should not be reached"),
                }
            }
            NodeEnum::Var(ref var) => Variable::String(var.clone()),
        }
    }
}

fn bool(nodes: &Vec<Variable>) -> Variable {
    Variable::Bool(parse_single_param(nodes))
}

fn int(nodes: &Vec<Variable>) -> Variable {
    Variable::Int(parse_single_param(nodes))
}
fn float(nodes: &Vec<Variable>) -> Variable {
    Variable::Float(parse_single_param(nodes))
}
fn add(nodes: &Vec<Variable>) -> Variable {
    let mut is_float = false;
    let mut sum: f32 = 0.;
    nodes.iter().for_each(|n| match *n {
        Variable::Float(f) => {
            sum += f;
            is_float = true;
        }
        Variable::Int(i) => sum += i as f32,
        _ => panic!("invalid type for Add"),
    });
    if is_float {
        return Variable::Float(sum);
    }
    return Variable::Int(sum as isize);
}

/// Subtracts one Variable from another.
fn sub(nodes: &Vec<Variable>) -> Variable {
    assert_eq!(nodes.len(), 2);
    let mut iter = nodes.iter();
    let v1 = iter.next().unwrap();
    let v2 = iter.next().unwrap();

    match v1 {
        Variable::Float(f) => {
            if let Variable::Float(f2) = v2 {
                return Variable::Float(f - f2);
            } else {
                panic!("second value is not float")
            }
        }
        Variable::Int(i) => {
            if let Variable::Int(i2) = v2 {
                return Variable::Int(i - i2);
            } else {
                panic!("second value is not float")
            }
        }
        _ => panic!("invalid type for Min"),
    }
}
/// Divides one Variable by another.
fn div(nodes: &Vec<Variable>) -> Variable {
    assert_eq!(nodes.len(), 2);
    let mut iter = nodes.iter();
    let v1 = iter.next().unwrap();
    let v2 = iter.next().unwrap();

    match v1 {
        Variable::Float(f) => {
            if let Variable::Float(f2) = v2 {
                return Variable::Float(f / f2);
            } else {
                panic!("second value is not float")
            }
        }
        Variable::Int(i) => {
            if let Variable::Int(i2) = v2 {
                return Variable::Int(i / i2);
            } else {
                panic!("second value is not float")
            }
        }
        _ => panic!("invalid type for Min"),
    }
}

/// Multiplies one Variable by another.
fn mult(nodes: &Vec<Variable>) -> Variable {
    assert_eq!(nodes.len(), 2);
    let mut iter = nodes.iter();
    let v1 = iter.next().unwrap();
    let v2 = iter.next().unwrap();

    match v1 {
        Variable::Float(f) => {
            if let Variable::Float(f2) = v2 {
                return Variable::Float(f * f2);
            } else {
                panic!("second value is not float")
            }
        }
        Variable::Int(i) => {
            if let Variable::Int(i2) = v2 {
                return Variable::Int(i * i2);
            } else {
                panic!("second value is not float")
            }
        }
        _ => panic!("invalid type for Min"),
    }
}

// Extracts field/index from wanted Variable.
fn extract(nodes: &Vec<Variable>) -> Variable {
    assert_eq!(nodes.len(), 2);
    let mut iter = nodes.iter();
    let v1 = iter.next().unwrap();
    let v2 = iter.next().unwrap();
    v1.extract(v2).unwrap()
}

// Defines new variable and writes it to a state.
fn define(nodes: &Vec<Variable>, state: &mut Engine) -> Variable {
    assert_eq!(nodes.len(), 2);
    let mut iter = nodes.iter();
    let v1 = iter.next().unwrap();
    let v2 = iter.next().unwrap().to_owned();

    if let Variable::String(s) = v1 {
        state.set(s.to_string(), v2);
    } else {
        panic!("variable name is not a Variable::String")
    }

    Variable::None
}

// Returns declared variable.
fn get(nodes: &Vec<Variable>, state: &Engine) -> Variable {
    let v = parse_single_param::<String>(nodes);

    state.get(v).clone()
}

// Returns Variable::Object parsed from json-like string.
fn object(nodes: &Vec<Variable>) -> Variable {
    let v = parse_single_param::<String>(nodes);

    let obj: Value = serde_json::from_str(&v).unwrap();
    assert!(obj.is_object());
    value_object_to_variable_object(obj)
}

// Returns Variable::Json.
fn json(nodes: &Vec<Variable>) -> Variable {
    let v = parse_single_param::<String>(nodes);
    let obj: Value = serde_json::from_str(&v).unwrap();
    Variable::Json(obj)
}

// Performs GET http request, returns Variable::Json.
fn http(nodes: &Vec<Variable>) -> Variable {
    let url = parse_single_param::<String>(nodes);
    let body: Value = reqwest::blocking::get(url).unwrap().json().unwrap();
    Variable::Json(body)
}

/// Parses single Variable to given type.
fn parse_single_param<T>(nodes: &Vec<Variable>) -> T
where
    T: std::str::FromStr + std::fmt::Debug,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    assert_eq!(nodes.len(), 1);
    let param = nodes.first().unwrap();
    parse_type(param)
}

/// Parses Variable to given type.
fn parse_type<T>(v: &Variable) -> T
where
    T: std::str::FromStr + std::fmt::Debug,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    if let Variable::String(s) = v {
        return s.parse::<T>().unwrap();
    }
    panic!("param is not Variable::String")
}

/// All supported keyword that can be used in steps declarations.
#[derive(PartialEq, Debug, Clone)]
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
    Min,
    Div,
    Mult,
    HTTP, // performs http request, has to return Variable::Json.
}

impl Keyword {
    fn from_string(s: &String) -> Option<Self> {
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
            "min" => Self::Min,
            "div" => Self::Div,
            "mult" => Self::Mult,
            "object" => Self::Object,
            "http" => Self::HTTP,
            _ => Self::None,
        };
        if s == Self::None {
            return None;
        }
        Some(s)
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
            text: text,
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
            self.current_token = self.tokens.iter().nth(self.token_inx).unwrap().clone();
        } else {
            self.done = true
        }
    }

    /// Creates Nodes tree from given Tokens.
    pub fn parse(&mut self) -> Node {
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
                    return pt;
                }
                Token::Var(ref v) => pt.push(Node::new_var(v.clone())),
                Token::Keyword(_) => pt.push(self.parse()),
                _ => {}
            }
        }
        pt
    }
}

#[cfg(test)]
mod tests {
    use super::Lexer;
    use crate::lang::{
        engine::{Definition, Engine},
        lexer::{Keyword, Node, Parser, Token},
        variable::Variable,
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
        let got = parser.parse();

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
        println!("{:?}", tokens);

        let mut parser = Parser::new(tokens);
        let got = parser.parse();
        println!("{:?}", got);

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
        println!("{:?}", tokens);
        let got = Parser::new(tokens).parse();

        let r = Node::new_keyword(Keyword::Define)
            .append(Node::new_var(String::from("var3")))
            .append(Node::new_var(String::from("qwdqw")));

        assert_eq!(got, r);
    }

    #[test]
    fn test_eval() {
        let mut state = Engine::default();

        let var = String::from("var");
        let n1 = Node::new_var(var.clone());
        assert_eq!(n1.eval(&mut state), Variable::String(var));

        let n1 = Node::new_keyword(Keyword::Bool).append(Node::new_var(String::from("true")));
        assert_eq!(n1.eval(&mut state), Variable::Bool(true));

        let n1 = Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.3")));
        assert_eq!(n1.eval(&mut state), Variable::Float(2.3));

        let n2 = Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("123")));
        assert_eq!(n2.eval(&mut state), Variable::Int(123));

        let n3 = Node::new_keyword(Keyword::Add)
            .append(n1)
            .append(n2.clone());
        assert_eq!(n3.eval(&mut state), Variable::Float(125.3));

        let n4 = Node::new_keyword(Keyword::Add)
            .append(n2.clone())
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("200"))));
        assert_eq!(n4.eval(&mut state), Variable::Int(323));

        let n5 = Node::new_keyword(Keyword::Min)
            .append(n2)
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("23"))));
        assert_eq!(n5.eval(&mut state), Variable::Int(100));

        let n5 = Node::new_keyword(Keyword::Div)
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("20"))))
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("2"))));
        assert_eq!(n5.eval(&mut state), Variable::Int(10));

        let n5 = Node::new_keyword(Keyword::Div)
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("20.0"))))
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"))));
        assert_eq!(n5.eval(&mut state), Variable::Float(8.));

        let n5 = Node::new_keyword(Keyword::Div)
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("-20"))))
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("2"))));
        assert_eq!(n5.eval(&mut state), Variable::Int(-10));

        let n5 = Node::new_keyword(Keyword::Div)
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("-20.0"))))
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"))));
        assert_eq!(n5.eval(&mut state), Variable::Float(-8.));

        let n5 = Node::new_keyword(Keyword::Mult)
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("-20.0"))))
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"))));
        assert_eq!(n5.eval(&mut state), Variable::Float(-50.));

        let n5 = Node::new_keyword(Keyword::Mult)
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("-20"))))
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("2"))));
        assert_eq!(n5.eval(&mut state), Variable::Int(-40));
    }

    #[test]
    fn test_eval_complex() {
        let mut state = Engine::default();

        let n1 = Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.3")));

        let n2 = Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("123")));
        let n3 = Node::new_keyword(Keyword::Add)
            .append(n1)
            .append(n2.clone());
        let n4 = Node::new_keyword(Keyword::Mult)
            .append(n3)
            .append(Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"))));
        assert_eq!(n4.eval(&mut state), Variable::Float(313.25));
    }

    #[test]
    fn test_parse_eval() {
        let mut state = Engine::default();

        let mut lexer = Lexer::new("VEC(1,BOOL(true),3,FLOAT(4.0))");
        let tokens = lexer.make_tokens();
        let mut parser = Parser::new(tokens);
        let got = parser.parse();
        assert_eq!(
            got.eval(&mut state),
            Variable::Vector(vec![
                Variable::String(String::from("1")),
                Variable::Bool(true),
                Variable::String(String::from("3")),
                Variable::Float(4.0)
            ])
        )
    }

    fn fire(def: Definition, state: &mut Engine) {
        def.into_iter().for_each(|s| {
            let tokens = Lexer::new(&s).make_tokens();
            let root = Parser::new(tokens).parse();
            root.eval(state);
        });
    }
    /// Runs single test scenario.
    fn test(def: Definition, var_name: String, value: Variable) {
        let mut state = Engine::default();
        fire(def, &mut state);
        assert_eq!(*state.get(var_name), value);
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
        map.insert(
            String::from("kid"),
            Variable::String(String::from("kidvalue")),
        );
        map.insert(String::from("kty"), Variable::String(String::from("RSA")));
        map.insert(String::from("use"), Variable::String(String::from("sig")));
        map.insert(String::from("n"), Variable::String(String::from("nvalue")));
        map.insert(String::from("e"), Variable::String(String::from("evalue")));

        let mut state = Engine::default();
        let def = Definition::new(vec![
            format!("DEFINE(var, OBJECT('{}'))", map_str).to_string(),
            String::from("DEFINE(var2, EXTRACT(GET(var), kty))"),
            String::from("DEFINE(var3, EXTRACT(GET(var), use))"),
            String::from("DEFINE(var4, EXTRACT(GET(var), n))"),
        ]);
        fire(def, &mut state);
        assert_eq!(*state.get(String::from("var")), Variable::Object(map));
        assert_eq!(
            *state.get(String::from("var2")),
            Variable::String(String::from("RSA"))
        );
        assert_eq!(
            *state.get(String::from("var3")),
            Variable::String(String::from("sig"))
        );
        assert_eq!(
            *state.get(String::from("var4")),
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
        let mut state = Engine::default();
        let def = Definition::new(vec![
            format!("DEFINE(var, object('{}'))", map_str).to_string(),
            String::from("DEFINE(var2, EXTRACT(GET(var), kty))"),
        ]);
        fire(def, &mut state);
        assert_eq!(*state.get(String::from("var")), Variable::Object(map));
        assert_eq!(*state.get(String::from("var2")), obj);
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

        let mut state = Engine::default();
        let def = Definition::new(vec![
            format!("DEFINE(var, JSON('{}'))", data).to_string(),
            "DEFINE(var2, EXTRACT(GET(var), name))".to_string(),
        ]);
        fire(def, &mut state);
        assert_eq!(*state.get("var".to_string()), Variable::Json(v));
        assert_eq!(
            *state.get("var2".to_string()),
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
}
