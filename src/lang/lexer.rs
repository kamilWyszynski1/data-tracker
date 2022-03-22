use std::{
    env::VarError,
    fmt::{self, Display},
};

use super::eval::Variable;

#[derive(Debug, PartialEq, Clone)]
enum NodeEnum {
    None,
    Keyword(Keyword),
    Var(String),
}

impl NodeEnum {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, PartialEq, Clone)]
struct Node {
    value: NodeEnum,
    nodes: Vec<Box<Node>>,
}

impl Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.value)
    }
}

impl Node {
    fn default() -> Self {
        Node {
            value: NodeEnum::default(),
            nodes: vec![],
        }
    }

    fn new_keyword(keyword: Keyword) -> Self {
        Node {
            value: NodeEnum::Keyword(keyword),
            nodes: vec![],
        }
    }

    fn new_var(var: String) -> Self {
        Node {
            value: NodeEnum::Var(var),
            nodes: vec![],
        }
    }

    fn add(&mut self, pt: Node) {
        self.nodes.push(Box::new(pt))
    }

    fn append(&mut self, n: Node) -> Self {
        self.add(n);
        self.clone()
    }

    fn eval(&self) -> Variable {
        match self.value {
            NodeEnum::None => todo!(),
            NodeEnum::Keyword(ref keyword) => {
                let nodes: Vec<Variable> = self.nodes.iter().map(|n| n.eval()).collect();
                match keyword {
                    Keyword::Bool => return Variable::Bool(parse_single_param(nodes)),
                    Keyword::Int => return Variable::Int(parse_single_param(nodes)),
                    Keyword::Float => return Variable::Float(parse_single_param(nodes)),
                    Keyword::Add => {
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
                    Keyword::Min => todo!(),
                    Keyword::Div => todo!(),
                    Keyword::Mult => todo!(),
                    _ => todo!(),
                }
            }
            NodeEnum::Var(ref var) => Variable::String(var.clone()),
        }
    }
}

fn parse_single_param<T>(nodes: Vec<Variable>) -> T
where
    T: std::str::FromStr + std::fmt::Debug,
    <T as std::str::FromStr>::Err: std::fmt::Debug,
{
    assert_eq!(nodes.len(), 1);
    let param = nodes.first().unwrap();
    if let Variable::String(s) = param {
        return s.parse::<T>().unwrap();
    }
    panic!("param is not Variable::String")
}

#[derive(PartialEq, Debug, Clone)]
enum Keyword {
    None,
    Define,
    Json,
    Vec,
    Extract,
    Get,
    Bool,
    Int,
    Float,
    Add,
    Min,
    Div,
    Mult,
}

impl Keyword {
    fn from_string(s: &String) -> Option<Self> {
        let s = match s.to_lowercase().as_str() {
            "define" => Self::Define,
            "json" => Self::Json,
            "vec" => Self::Vec,
            "extract" => Self::Extract,
            "get" => Self::Get,
            "bool" => Self::Bool,
            "int" => Self::Int,
            "float" => Self::Float,
            "add" => Self::Add,
            "min" => Self::Min,
            "div" => Self::Div,
            "mult" => Self::Mult,

            _ => Self::None,
        };
        if s == Self::None {
            return None;
        }
        Some(s)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    LeftBracket,
    RightBracket,
    Comma,
    Keyword(Keyword),
    Var(String),
}
struct Lexer {
    text: String,
    pos: usize,
    current_char: char,
    done: bool,
}

impl Lexer {
    fn new(text: String) -> Self {
        Lexer {
            text: text.clone(),
            pos: 0,
            current_char: text.chars().next().unwrap(),
            done: false,
        }
    }

    fn advance(&mut self) {
        self.pos += 1;
        if self.pos < self.text.len() - 1 {
            self.current_char = self.text.chars().nth(self.pos).unwrap();
        } else {
            self.done = true
        }
    }

    fn make_tokens(&mut self) -> Vec<Token> {
        let mut tokens: Vec<Token> = vec![];

        while !self.done {
            if self.current_char.is_whitespace() {
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

        while !self.done && (self.current_char.is_alphanumeric() || self.current_char == '.') {
            word.push(self.current_char);
            self.advance();
        }

        if let Some(f) = Keyword::from_string(&word) {
            Token::Keyword(f)
        } else {
            Token::Var(word)
        }
    }
}

struct Parser {
    tokens: Vec<Token>,
    token_inx: usize,
    current_token: Token,
    done: bool,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
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

    // "DEFINE(var, VEC(1,BOOL(2),3,FLOAT(4.0)))"
    fn parse(&mut self) -> Node {
        let mut pt: Node;
        if let Token::Keyword(s) = &self.current_token {
            pt = Node::new_keyword(s.clone());
        } else {
            dbg!(&self.current_token);
            panic!("first token is not keyword")
        }
        while !self.done {
            self.advance();
            match self.current_token {
                Token::RightBracket => {
                    return pt;
                }
                Token::Var(ref v) => pt.add(Node::new_var(v.clone())),
                Token::Keyword(ref kw) => pt.add(self.parse()),
                _ => {}
            }
        }
        pt
    }
}

#[cfg(test)]
mod tests {
    use crate::lang::{
        eval::Variable,
        lexer::{Keyword, Node, Parser},
    };

    use super::Lexer;

    #[test]
    fn test_lexer() {
        let mut lexer = Lexer::new(String::from("DEFINE(var, VEC(1,BOOL(2),3,FLOAT(4.0)))"));
        let tokens = lexer.make_tokens();
        println!("{:?}", tokens);

        let mut parser = Parser::new(tokens);
        let got = parser.parse();
        println!("{:?}", got);

        let mut main = Node::new_keyword(Keyword::Define);
        main.add(Node::new_var(String::from("var")));
        let mut vec = Node::new_keyword(Keyword::Vec);
        let v1 = Node::new_var(String::from("1"));
        let v2 = Node::new_keyword(Keyword::Bool).append(Node::new_var(String::from("2")));
        let v3 = Node::new_var(String::from("3"));
        let v4 = Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("4.0")));
        vec.add(v1);
        vec.add(v2);
        vec.add(v3);
        vec.add(v4);
        main.add(vec);
        assert_eq!(got, main);
    }

    #[test]
    fn test_lexer_v2() {
        let mut lexer = Lexer::new(String::from("DEFINE(var3, EXTRACT(var, use))"));
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
    fn test_eval() {
        let var = String::from("var");
        let n1 = Node::new_var(var.clone());
        assert_eq!(n1.eval(), Variable::String(var));

        let n1 = Node::new_keyword(Keyword::Bool).append(Node::new_var(String::from("true")));
        assert_eq!(n1.eval(), Variable::Bool(true));

        let n1 = Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.3")));
        assert_eq!(n1.eval(), Variable::Float(2.3));

        let n2 = Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("123")));
        assert_eq!(n2.eval(), Variable::Int(123));

        let n3 = Node::new_keyword(Keyword::Add)
            .append(n1)
            .append(n2.clone());
        assert_eq!(n3.eval(), Variable::Float(125.3));

        let n4 = Node::new_keyword(Keyword::Add)
            .append(n2)
            .append(Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("200"))));
        assert_eq!(n4.eval(), Variable::Int(323));
    }
}
