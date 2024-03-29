use super::node::{Node, NodeEnum};
use super::variable::Variable;
use crate::error::types::{Error, Result};
use core::panic;
use serde::{Deserialize, Serialize};

/// All supported keyword that can be used in steps declarations.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum Keyword {
    None,
    /// default value, not supported by language.
    Define,
    /// defines new variable: DEFINE(var, 1).
    Get,
    /// returns defined variable: VAR(var).
    Extract,
    /// appends data to given variable.
    /// supported variables to be appended: String, Vec, Object.
    Append,

    Json,
    /// returns Variable::Json: JSON("{}").
    Vec,
    /// returns Variable::Vec: VEC(1,2,3,4).
    Object,
    /// returns Variable::Object: Object(a, 1, b, INT(3), c, BOOL(true))
    Bool,
    Int,
    Float,

    Add,
    Sub,
    Div,
    Mult,

    HTTP,
    /// performs http request, has to return Variable::Json.
    Log,
    /// logs given Variable.
    RunSubtree,
    /// takes 1 argument, subtree name: RunSubtree(subtree_name).

    /// Takes no arguments, breaks from RunSubtree.
    /// If RunSubtree are nested it'll break to root point.
    Break,
    /// conditional run: IF(BOOL(true), RunSubtree(subtree_name)).
    If,
    /// Returns true if two Variable are equal.
    /// Can be chained like that: Eq(Eq(INT(1), INT(1)), Eq(FLOAT(2.5), FLOAT(2.5))).
    Eq,
    Neq,

    Map,
    /// Can be used for vector/object values mapping: MAP(VEC(1,2,3), ADD(x, INT(4)))
    MapInPlace,
    /// Works same as Map but do not return variable, modifies given one.
    Filter,
    /// Can be used for vector/object values filtering: FILTER(VEC(1,2,3), EQ(X, 2)))

    /// Takes 1 argument - alias to mounted resource, reads its content as a string.
    ReadMountedToString,

    /// Starts transactions. Every modifications on data after Begin will be inside transcations
    /// which means that it won't be applied to final variable state till Commit/Rollback is called.
    ///
    /// Only one transaction can be started at a time.
    Begin,
    /// Applies transaction changes to real state.
    ///
    /// Fails if no transaction was started.
    Commit,
    /// Discards transaction changes.
    ///
    /// Fails if no transaction was started.
    Rollback,
}

impl Keyword {
    fn from_string(s: &str) -> Option<Self> {
        let s = match s.to_lowercase().as_str() {
            "define" => Self::Define,
            "get" => Self::Get,
            "json" => Self::Json,
            "vec" => Self::Vec,
            "extract" => Self::Extract,
            "append" => Self::Append,
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
            "mapinplace" => Self::MapInPlace,
            "filter" => Self::Filter,
            "readmountedtostring" => Self::ReadMountedToString,
            "begin" => Self::Begin,
            "commit" => Self::Commit,
            "rollback" => Self::Rollback,
            _ => Self::None,
        };
        if s == Self::None {
            return None;
        }
        Some(s)
    }

    /// Returns error if there's invalid number of arguments for given Keyword.
    pub fn check_arguments_count(&self, nodes: &[Variable]) -> Result<()> {
        let wanted = match self {
            Keyword::None
            | Keyword::Break
            | Keyword::Begin
            | Keyword::Commit
            | Keyword::Rollback => 0,
            Keyword::Get
            | Keyword::Json
            | Keyword::Object
            | Keyword::Bool
            | Keyword::Int
            | Keyword::Float
            | Keyword::HTTP
            | Keyword::Log
            | Keyword::RunSubtree
            | Keyword::ReadMountedToString => 1,
            Keyword::Define
            | Keyword::Add
            | Keyword::Sub
            | Keyword::Div
            | Keyword::Mult
            | Keyword::If
            | Keyword::Eq
            | Keyword::Neq
            | Keyword::Map
            | Keyword::MapInPlace
            | Keyword::Filter
            | Keyword::Append => 2,
            Keyword::Vec => {
                if nodes.is_empty() {
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
            .then_some(0)
            .ok_or_else(|| {
                Error::new_eval_internal(
                    String::from("Keyword::check_arguments_count"),
                    format!(
                        "keyword: {:?} - wanted {} arguments, got {}",
                        self,
                        wanted,
                        nodes.len()
                    ),
                )
            })
            .map(|_| ())
    }

    /// Determines if enum variant's first argument is l_value or not.
    /// l_value won't be automatically converted into initialized variable with the same name.
    fn contain_l_value(&self) -> bool {
        matches!(self, Self::Define | Self::Get)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents one independent piece of declaration.
pub enum Token {
    LeftBracket,
    RightBracket,
    Comma,
    Keyword(Keyword),

    /// Contains string value of a parsed token and l_value indicator.
    /// If something is l_value, it won't be treated as initialized variable.
    Var {
        value: String,
        l_value: bool,
    },
}

/// Takes care of creating Tokens from wanted declaration.
pub struct Lexer<'a> {
    text: &'a str,
    pos: usize,
    current_char: char,
    done: bool,

    /// Contains previously parsed token, handy for determining if something is l_value or not.
    /// Check Keyword::contain_l_value method for more info.
    previous: Option<Token>,
}

impl<'a> Lexer<'a> {
    pub fn new(text: &'a str) -> Self {
        Lexer {
            text,
            pos: 0,
            current_char: text.chars().next().unwrap(),
            done: false,
            previous: None,
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

        // will be set to 'true' whenever we find '(' which indicates that next value is a l_value.

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

        let mut token = Token::Var {
            value: word.clone(),
            l_value: false,
        };
        if let Some(f) = Keyword::from_string(&word) {
            token = Token::Keyword(f)
        } else if let Some(Token::Keyword(k)) = &self.previous {
            if k.contain_l_value() || apostrophe_found {
                token = Token::Var {
                    value: word,
                    l_value: true,
                };
            }
        };
        self.previous = Some(token.clone());
        token
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
            match &self.current_token {
                Token::RightBracket => {
                    break;
                }
                Token::Keyword(_) => {
                    if let Some(parsed) = self.parse() {
                        pt.push(parsed)
                    }
                }
                Token::Var { value, l_value } => pt.push(Node::new_var(value.clone(), *l_value)),
                _ => (),
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
            engine::Engine,
            eval::EvalForest,
            lexer::{Keyword, Node, NodeEnum, Parser, Token},
            node::{EvalMetadata, SharedState},
            process::{Definition, Process, SubTree},
            variable::Variable,
        },
    };
    use serde_json::Value;
    use std::collections::HashMap;

    /// helper function to run fast evaluations.
    fn evaluate(in_var: Option<Variable>, eval_forest: &EvalForest) -> Result<Variable> {
        let mut variables = HashMap::new();

        if let Some(variable) = in_var {
            variables.insert(String::from("IN"), variable.clone());
            variables.insert(String::from("OUT"), variable);
        }

        let mut shared_state = SharedState::new(variables, eval_forest.subtrees.clone());

        for root in &eval_forest.roots {
            root.start_evaluation(&mut shared_state)?;
        }

        shared_state
            .variables
            .get("OUT")
            .ok_or_else(|| {
                Error::new_eval_internal(
                    String::from("evaluate"),
                    String::from("failed to get 'OUT' variable"),
                )
            })
            .map(|v| v.clone())
    }

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
            Token::Var {
                value: map_str.to_string(),
                l_value: true,
            },
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
            Token::Var {
                value: String::from("var"),
                l_value: true,
            },
            Token::Comma,
            Token::Keyword(Keyword::Vec),
            Token::LeftBracket,
            Token::Var {
                value: String::from("1"),
                l_value: false,
            },
            Token::Comma,
            Token::Keyword(Keyword::Bool),
            Token::LeftBracket,
            Token::Var {
                value: String::from("2"),
                l_value: false,
            },
            Token::RightBracket,
            Token::Comma,
            Token::Var {
                value: String::from("3"),
                l_value: false,
            },
            Token::Comma,
            Token::Keyword(Keyword::Float),
            Token::LeftBracket,
            Token::Var {
                value: String::from("4.0"),
                l_value: false,
            },
            Token::RightBracket,
            Token::RightBracket,
            Token::RightBracket,
        ];
        assert_eq!(tokens, wanted);

        let mut parser = Parser::new(tokens);
        let got = parser.parse().unwrap();

        let mut main = Node::new_keyword(Keyword::Define);
        main.push(Node::new_var(String::from("var"), true));
        let mut vec = Node::new_keyword(Keyword::Vec);
        let v1 = Node::new_var(String::from("1"), false);
        let v2 = Node::new_keyword(Keyword::Bool).append(Node::new_var(String::from("2"), false));
        let v3 = Node::new_var(String::from("3"), false);
        let v4 =
            Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("4.0"), false));
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
            .append(Node::new_var(String::from("var3"), true))
            .append(
                Node::new_keyword(Keyword::Extract)
                    .append(Node::new_var(String::from("var"), false))
                    .append(Node::new_var(String::from("use"), false)),
            );
        assert_eq!(got, main);
    }

    #[test]
    fn test_lexer_v3() {
        let tokens = Lexer::new("DEFINE(var3, qwdqw)").make_tokens();
        let got = Parser::new(tokens).parse().unwrap();

        let r = Node::new_keyword(Keyword::Define)
            .append(Node::new_var(String::from("var3"), true))
            .append(Node::new_var(String::from("qwdqw"), false));

        assert_eq!(got, r);
    }

    #[test]
    fn test_eval() {
        let var = String::from("var");
        let n1 = Node::new_var(var.clone(), false);

        let mut state = SharedState::default();
        assert_eq!(
            n1.start_evaluation(&mut state).unwrap(),
            Variable::String(var)
        );

        let n1 =
            Node::new_keyword(Keyword::Bool).append(Node::new_var(String::from("true"), false));
        assert_eq!(
            n1.start_evaluation(&mut state).unwrap(),
            Variable::Bool(true)
        );

        let n1 =
            Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.3"), false));
        assert_eq!(
            n1.start_evaluation(&mut state).unwrap(),
            Variable::Float(2.3)
        );

        let n2 = Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("123"), false));
        assert_eq!(n2.start_evaluation(&mut state).unwrap(), Variable::Int(123));

        let n3 = Node::new_keyword(Keyword::Add)
            .append(n1)
            .append(n2.clone());
        assert_eq!(
            n3.start_evaluation(&mut state).unwrap(),
            Variable::Float(125.3)
        );

        let n4 = Node::new_keyword(Keyword::Add).append(n2.clone()).append(
            Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("200"), false)),
        );
        assert_eq!(n4.start_evaluation(&mut state).unwrap(), Variable::Int(323));

        let n5 = Node::new_keyword(Keyword::Sub).append(n2).append(
            Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("23"), false)),
        );
        assert_eq!(n5.start_evaluation(&mut state).unwrap(), Variable::Int(100));

        let n5 = Node::new_keyword(Keyword::Div)
            .append(
                Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("20"), false)),
            )
            .append(
                Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("2"), false)),
            );
        assert_eq!(n5.start_evaluation(&mut state).unwrap(), Variable::Int(10));

        let n5 = Node::new_keyword(Keyword::Div)
            .append(
                Node::new_keyword(Keyword::Float)
                    .append(Node::new_var(String::from("20.0"), false)),
            )
            .append(
                Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"), false)),
            );
        assert_eq!(
            n5.start_evaluation(&mut state).unwrap(),
            Variable::Float(8.)
        );

        let n5 = Node::new_keyword(Keyword::Div)
            .append(
                Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("-20"), false)),
            )
            .append(
                Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("2"), false)),
            );
        assert_eq!(n5.start_evaluation(&mut state).unwrap(), Variable::Int(-10));

        let n5 = Node::new_keyword(Keyword::Div)
            .append(
                Node::new_keyword(Keyword::Float)
                    .append(Node::new_var(String::from("-20.0"), false)),
            )
            .append(
                Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"), false)),
            );
        assert_eq!(
            n5.start_evaluation(&mut state).unwrap(),
            Variable::Float(-8.)
        );

        let n5 = Node::new_keyword(Keyword::Mult)
            .append(
                Node::new_keyword(Keyword::Float)
                    .append(Node::new_var(String::from("-20.0"), false)),
            )
            .append(
                Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"), false)),
            );
        assert_eq!(
            n5.start_evaluation(&mut state).unwrap(),
            Variable::Float(-50.)
        );

        let n5 = Node::new_keyword(Keyword::Mult)
            .append(
                Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("-20"), false)),
            )
            .append(
                Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("2"), false)),
            );
        assert_eq!(n5.start_evaluation(&mut state).unwrap(), Variable::Int(-40));
    }

    #[test]
    fn test_eval_complex() {
        let mut state = SharedState::default();

        let n1 =
            Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.3"), false));

        let n2 = Node::new_keyword(Keyword::Int).append(Node::new_var(String::from("123"), false));
        let n3 = Node::new_keyword(Keyword::Add).append(n1).append(n2);
        let n4 = Node::new_keyword(Keyword::Mult).append(n3).append(
            Node::new_keyword(Keyword::Float).append(Node::new_var(String::from("2.5"), false)),
        );
        assert_eq!(
            n4.start_evaluation(&mut state).unwrap(),
            Variable::Float(313.25)
        );
    }

    #[test]
    fn test_parse_eval() {
        let mut state = SharedState::default();

        let mut lexer = Lexer::new("VEC(1,BOOL(true),3,FLOAT(4.0))");
        let tokens = lexer.make_tokens();
        let mut parser = Parser::new(tokens);
        let got = parser.parse().unwrap();
        println!("{:?}", got);
        assert_eq!(
            got.start_evaluation(&mut state).unwrap(),
            Variable::Vector(vec![
                Variable::String(String::from("1")),
                Variable::Bool(true),
                Variable::String(String::from("3")),
                Variable::Float(4.0)
            ])
        )
    }

    fn fire_for_test(def: Definition, state: &mut SharedState) -> Result<()> {
        for step in def {
            let root = Parser::new(Lexer::new(&step).make_tokens())
                .parse()
                .unwrap();
            root.start_evaluation(state)?;
        }
        Ok(())
    }
    /// Runs single test scenario.
    fn test(def: Definition, var_name: String, value: Variable) {
        let mut state = SharedState::default();

        fire_for_test(def, &mut state).unwrap();
        assert_eq!(*state.variables.get(&var_name).unwrap(), value);
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

        let mut state = SharedState::default();

        let def = Definition::new(vec![
            format!("DEFINE(var, OBJECT('{}'))", map_str),
            String::from("DEFINE(var2, EXTRACT(GET(var), kty))"),
            String::from("DEFINE(var3, EXTRACT(GET(var), use))"),
            String::from("DEFINE(var4, EXTRACT(GET(var), n))"),
        ]);
        fire_for_test(def, &mut state).unwrap();
        assert_eq!(*state.variables.get("var").unwrap(), Variable::Object(map));
        assert_eq!(
            *state.variables.get("var2").unwrap(),
            Variable::String(String::from("RSA"))
        );
        assert_eq!(
            *state.variables.get("var3").unwrap(),
            Variable::String(String::from("sig"))
        );
        assert_eq!(
            *state.variables.get("var4").unwrap(),
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

        let mut state = SharedState::default();

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

        let def = Definition::new(vec![
            format!("DEFINE(var, object('{}'))", map_str),
            String::from("DEFINE(var2, EXTRACT(GET(var), kty))"),
        ]);
        fire_for_test(def, &mut state).unwrap();
        assert_eq!(*state.variables.get("var").unwrap(), Variable::Object(map));
        assert_eq!(*state.variables.get("var2").unwrap(), obj);
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

        let mut state = SharedState::default();

        let def = Definition::new(vec![
            format!("DEFINE(var, JSON('{}'))", data),
            "DEFINE(var2, EXTRACT(GET(var), name))".to_string(),
        ]);
        fire_for_test(def, &mut state).unwrap();
        assert_eq!(*state.variables.get("var").unwrap(), Variable::Json(v));
        assert_eq!(
            *state.variables.get("var2").unwrap(),
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
        )]);
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

                definition: Definition::new(vec!["DEFINE(OUT, ADD(GET(IN), INT(10)))"]),
            }]),
            name: None,
            implicit_subtrees: None,
        };
        let eval_forest = EvalForest::from(definition);
        let out = evaluate(None, &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(30));

        let definition = Definition {
            steps: vec![String::from("RunSubtree(testsubtree)")],
            subtrees: Some(vec![
                SubTree {
                    name: String::from("testsubtree"),
                    definition: Definition::new(vec!["RunSubtree(testsubtree2)"]),
                },
                SubTree {
                    name: String::from("testsubtree2"),
                    definition: Definition {
                        steps: vec![String::from("DEFINE(OUT, INT(400))")],
                        subtrees: None,
                        name: None,
                        implicit_subtrees: None,
                    },
                },
            ]),
            name: None,
            implicit_subtrees: None,
        };
        let eval_forest = EvalForest::from(definition);
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
    //
    //             definition: Definition {
    //                 steps: vec![String::from("DEFINE(OUT, ADD(GET(IN), INT(10)))")],
    //                 subtrees: None,
    //             },
    //         }]),
    //     };

    //     let eval_forest = EvalForest::from(definition);
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
    //
    //                 definition: Definition {
    //                     steps: vec![String::from("DEFINE(OUT, ADD(GET(IN), INT(10)))")],
    //                     subtrees: None,
    //                 },
    //             },
    //             SubTree {
    //                 name: String::from("testsubtree2"),
    //
    //                 definition: Definition {
    //                     steps: vec![String::from("DEFINE(OUT, MULT(GET(IN), INT(10)))")],
    //                     subtrees: None,
    //                 },
    //             },
    //         ]),
    //     };

    //     let eval_forest = EvalForest::from(definition);
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
    //
    //             definition: Definition {
    //                 steps: vec![String::from("DEFINE(OUT, FLOAT(1.0))")],
    //                 subtrees: None,
    //             },
    //         }]),
    //     };

    //     let eval_forest = EvalForest::from(definition);
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
        env_logger::try_init().ok();

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
                definition: Definition {
                    steps: vec![String::from("DEFINE(OUT, ADD(GET(IN), INT(10)))")],
                    subtrees: None,
                    name: None,
                    implicit_subtrees: None,
                },
            }]),
            name: None,
            implicit_subtrees: None,
        };

        let eval_forest = EvalForest::from(definition);
        let out = evaluate(Some(Variable::Int(125)), &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(135));

        let definition = Definition {
            steps: vec![String::from("IF(INT(2), RunSubtree(testsubtree))")],
            subtrees: Some(vec![SubTree {
                name: String::from("testsubtree"),

                definition: Definition {
                    steps: vec![String::from("DEFINE(OUT, ADD(GET(IN), INT(10)))")],
                    subtrees: None,
                    name: None,
                    implicit_subtrees: None,
                },
            }]),
            name: None,
            implicit_subtrees: None,
        };

        let eval_forest = EvalForest::from(definition);
        let out = evaluate(Some(Variable::Int(125)), &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(125));

        let definition = Definition {
            steps: vec![String::from("RunSubtree(testsubtree)")],
            subtrees: Some(vec![
                SubTree {
                    name: String::from("testsubtree"),
                    definition: Definition {
                        steps: vec![String::from("IF(GET(IN), RunSubtree(testsubtree2))")],
                        subtrees: None,
                        name: None,
                        implicit_subtrees: None,
                    },
                },
                SubTree {
                    name: String::from("testsubtree2"),

                    definition: Definition {
                        steps: vec![String::from("DEFINE(OUT, INT(155))")],
                        subtrees: None,
                        name: None,
                        implicit_subtrees: None,
                    },
                },
            ]),
            name: None,
            implicit_subtrees: None,
        };

        let eval_forest = EvalForest::from(definition);
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
            name: None,
            implicit_subtrees: None,
        };

        let process = Process::new("test", vec![definition], None);
        let mut engine = Engine::new(Variable::Int(10), process).expect("could not create engine");

        engine.fire().expect("fire failed");
        assert_eq!(engine.get("var").unwrap(), &Variable::Int(100));

        let definition = Definition {
            steps: vec![String::from("IF(INT(123), DEFINE(var, INT(100)))")],
            subtrees: None,
            name: None,
            implicit_subtrees: None,
        };

        let process = Process::new("test", vec![definition], None);
        let mut engine = Engine::new(Variable::Int(10), process).expect("could not create engine");

        engine.fire().expect("fire failed");
        assert!(engine.get("var").is_none());

        let definition = Definition {
            steps: vec![String::from("IF(BOOL(true), DEFINE(var, INT(100)))")],
            subtrees: None,
            name: None,
            implicit_subtrees: None,
        };

        let process = Process::new("test", vec![definition], None);
        let mut engine = Engine::new(Variable::Int(10), process).expect("could not create engine");

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
        env_logger::try_init().ok();
        let definition = Definition {
            steps: vec![String::from("RunSubtree(testsubtree)")],
            subtrees: Some(vec![
                SubTree {
                    name: String::from("testsubtree"),
                    definition: Definition {
                        steps: vec![
                            String::from("BREAK"),
                            String::from("RunSubtree(testsubtree2)"),
                        ],
                        subtrees: None,
                        name: None,
                        implicit_subtrees: None,
                    },
                },
                SubTree {
                    name: String::from("testsubtree2"),
                    definition: Definition {
                        steps: vec![String::from("DEFINE(OUT, INT(155))")],
                        subtrees: None,
                        name: None,
                        implicit_subtrees: None,
                    },
                },
            ]),
            name: None,
            implicit_subtrees: None,
        };

        let eval_forest = EvalForest::from(definition);
        let out = evaluate(Some(Variable::Int(2)), &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(2));

        let definition = Definition {
            steps: vec![String::from("RunSubtree(testsubtree)")],
            subtrees: Some(vec![SubTree {
                name: String::from("testsubtree"),
                definition: Definition {
                    steps: vec![
                        String::from("DEFINE(IN, SUB(GET(IN), INT(1)))"),
                        String::from("IF(GET(IN), DEFINE(OUT, GET(IN)))"),
                        String::from("IF(GET(IN), BREAK)"),
                        String::from("RunSubtree(testsubtree)"),
                    ],
                    subtrees: None,
                    name: None,
                    implicit_subtrees: None,
                },
            }]),
            name: None,
            implicit_subtrees: None,
        };

        let eval_forest = EvalForest::from(definition);
        let out = evaluate(Some(Variable::Int(5)), &eval_forest).expect("could not evaluate");
        assert_eq!(out, Variable::Int(1));

        let definition = Definition {
            steps: vec![String::from("RunSubtree(testsubtree)")],
            subtrees: Some(vec![SubTree {
                name: String::from("testsubtree"),
                definition: Definition {
                    steps: vec![
                        String::from("DEFINE(IN, SUB(GET(IN), INT(1)))"),
                        String::from("RunSubtree(testsubtree)"),
                    ],
                    subtrees: None,
                    name: None,
                    implicit_subtrees: None,
                },
            }]),
            name: None,
            implicit_subtrees: None,
        };

        let eval_forest = EvalForest::from(definition);
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

    #[test]
    fn variable_as_default_test() {
        let def = Definition::new(vec![
            "DEFINE(OUT, VEC(INT(1), INT(2), INT(3)))",
            "DEFINE(OUT, EXTRACT(OUT, 0))",
        ]);
        test(def, String::from("OUT"), Variable::Int(1));
    }

    #[test]
    fn test_transactions() {
        let def = Definition::new(vec![
            "DEFINE(OUT, INT(1))",
            "BEGIN()",
            "DEFINE(OUT2, INT(2))",
            "ROLLBACK()",
            "BEGIN()",
            "DEFINE(OUT2, INT(3))",
            "COMMIT()",
        ]);

        let mut state = SharedState::default();

        fire_for_test(def, &mut state).unwrap();

        assert_eq!(state.variables.get("OUT").unwrap(), &Variable::Int(1));
        assert_eq!(state.variables.get("OUT2").unwrap(), &Variable::Int(3));

        let def = Definition::new(vec!["BEGIN()", "BEGIN()"]);

        assert_eq!(
            fire_for_test(def, &mut SharedState::default()),
            Err(Error::new_eval_internal(
                "begin",
                "transaction already started"
            ))
        );

        let def = Definition::new(vec!["COMMIT()"]);

        assert_eq!(
            fire_for_test(def, &mut SharedState::default()),
            Err(Error::new_eval_internal(
                "commit",
                "transaction wasn't started"
            ))
        );

        let def = Definition::new(vec!["ROLLBACK()"]);

        assert_eq!(
            fire_for_test(def, &mut SharedState::default()),
            Err(Error::new_eval_internal(
                "rollback",
                "transaction wasn't started"
            ))
        );
    }
}
