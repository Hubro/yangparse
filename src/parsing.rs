use regex::Regex;

use crate::constants::STATEMENT_KEYWORDS;
use crate::lexing::{Token, TokenType};

lazy_static! {
    /// See "identifier" from ABNF
    static ref IDENTIFIER_PATTERN: Regex = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9\-_.]*$").unwrap();

    /// identifier ":" identifier - See "unknown-statement" from ABNF
    static ref EXT_KEYWORD_PATTERN: Regex =
        Regex::new(r"^[a-zA-Z_][a-zA-Z0-9\-_.]*:[a-zA-Z_][a-zA-Z0-9\-_.]*$").unwrap();
}

#[derive(Debug)]
pub enum StatementKeyword {
    Keyword(String),
    ExtensionKeyword(String),
    Invalid(String),
}

#[derive(Debug)]
pub enum Node {
    BlockNode(BlockNode),
    LeafNode(LeafNode),
    CommentNode(String),
}

#[derive(Debug)]
pub struct RootNode {
    pub children: Vec<Node>,
}

#[derive(Debug)]
pub struct BlockNode {
    pub keyword: StatementKeyword,
    pub value: Option<NodeValue>,
    pub children: Vec<Node>,
}

#[derive(Debug)]
pub struct LeafNode {
    pub keyword: StatementKeyword,
    pub value: NodeValue,
}

/// The value of a node, currently simply represented as a
#[derive(Debug)]
pub enum NodeValue {
    String(String),
    Number(String),
    Date(String),

    /// Any value not obviously identifiable as a quoted string, number or date is just loosely
    /// categorized as "other". This can be extended to support more fine grained types such as
    /// identifiers, booleans, xpaths, keypaths and so on if a use-case arrives for it (such as
    /// linting).
    Other(String),
}

enum ParseState {
    Clean,
    GotKeyword(StatementKeyword),
    GotValue(StatementKeyword, NodeValue),
}

impl From<&Token<'_>> for StatementKeyword {
    fn from(token: &Token) -> Self {
        if STATEMENT_KEYWORDS.contains(&token.text) {
            StatementKeyword::Keyword(token.text.to_string())
        } else if EXT_KEYWORD_PATTERN.is_match(token.text) {
            StatementKeyword::ExtensionKeyword(token.text.to_string())
        } else {
            // Anything that is not a statement keyword or an extension keyword is invalid, but
            // we'll keep building the tree anyway.
            StatementKeyword::Invalid(token.text.to_string())
        }
    }
}

impl From<Token<'_>> for StatementKeyword {
    fn from(token: Token) -> Self {
        (&token).into()
    }
}

impl From<&Token<'_>> for NodeValue {
    fn from(token: &Token) -> Self {
        match token.token_type {
            TokenType::String => Self::String(token.text.to_string()),
            TokenType::Number => Self::Number(token.text.to_string()),
            TokenType::Date => Self::Date(token.text.to_string()),
            _ => Self::Other(token.text.to_string()),
        }
    }
}

impl From<Token<'_>> for NodeValue {
    fn from(token: Token) -> Self {
        (&token).into()
    }
}

/// Parses the input bytes as a YANG documents and returns a syntax tree
///
/// The returned node is a virtual "root" block node. This node contains the actual module or
/// sub-module node as one of its children, as well as any comments that are above or below that
/// node.
///
/// This parser doesn't strictly enforce the official grammar, and the returned tree may well be
/// invalid YANG. For example, this function will parse a document with multiple module blocks just
/// fine, or no module node at all, just a bunch of leafs.
///
pub fn parse(buffer: &[u8]) -> Result<RootNode, String> {
    let mut tokens = crate::lexing::scan(buffer);

    Ok(RootNode {
        children: parse_statements(&mut tokens)?,
    })
}

fn parse_statements(tokens: &mut crate::lexing::ScanIterator) -> Result<Vec<Node>, String> {
    let mut statements: Vec<Node> = vec![];
    let mut state = ParseState::Clean;

    loop {
        match tokens.next() {
            Some(token) => {
                match state {
                    ParseState::Clean => {
                        // From a clean state, we expect to find a statement keyword, a comment or
                        // a closing curly brace
                        match token.token_type {
                            TokenType::WhiteSpace => continue,
                            TokenType::LineBreak => continue,
                            TokenType::Comment => {
                                statements.push(Node::CommentNode(token.text.to_string()))
                            }
                            TokenType::ClosingCurlyBrace => {
                                return Ok(statements);
                            }
                            TokenType::Other => state = ParseState::GotKeyword(token.into()),
                            _ => return Err(format!("Unexpected token: {:?}", token)),
                        }
                    }

                    ParseState::GotKeyword(keyword) => {
                        match token.token_type {
                            TokenType::WhiteSpace => state = ParseState::GotKeyword(keyword),
                            TokenType::LineBreak => state = ParseState::GotKeyword(keyword),

                            TokenType::OpenCurlyBrace => {
                                // Recurse!
                                statements.push(Node::BlockNode(BlockNode {
                                    keyword,
                                    value: None,
                                    children: parse_statements(tokens)?,
                                }));

                                state = ParseState::Clean;
                            }

                            TokenType::SemiColon => {
                                return Err("Expected to find a value, not \";\"".to_string());
                            }

                            _ => {
                                state = ParseState::GotValue(keyword, token.into());
                            }
                        }
                    }

                    ParseState::GotValue(keyword, value) => {
                        match token.token_type {
                            TokenType::WhiteSpace => state = ParseState::GotValue(keyword, value),
                            TokenType::LineBreak => state = ParseState::GotValue(keyword, value),

                            TokenType::OpenCurlyBrace => {
                                // Recurse!
                                statements.push(Node::BlockNode(BlockNode {
                                    keyword,
                                    value: Some(value),
                                    children: parse_statements(tokens)?,
                                }));

                                state = ParseState::Clean;
                            }

                            TokenType::SemiColon => {
                                statements.push(Node::LeafNode(LeafNode { keyword, value }));

                                state = ParseState::Clean;
                            }

                            _ => {
                                return Err(format!(
                                    "Expected semicolon or block, got: {:?}",
                                    token
                                ));
                            }
                        }
                    }
                }
            }

            // When we reach the end of the token stream, we're done and can return
            None => match state {
                ParseState::Clean => return Ok(statements),
                _ => return Err("Unexpected end of input".to_string()),
            },
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use pretty_assertions::assert_eq;

    fn dedent(text: &str) -> String {
        let mut text = textwrap::dedent(text).trim().to_string();
        text.push('\n');
        text
    }

    #[test]
    fn smoke_test() {
        let buffer: Vec<u8> = dedent(
            r#"
             /*
              * This is a block comment
              */

             module test {
                 yang-version 1;
                 namespace "https://github.com/Hubro/yangparse";
                 description 'A small smoke test to make sure basic lexing works';

                 revision 2018-12-03 {
                     // I'm a comment!
                     description
                       "A multi-line string starting in an indented line

                        This is an idiomatic way to format large strings
                        in YANG models";
                 }

                 number 12.34;
             }
             "#,
        )
        .bytes()
        .collect();

        let tree = parse(&buffer).expect("Failed to parse YANG");

        assert_eq!(
            dedent(
                r#"
                (root
                  (comment)
                  (Keyword "module" Other
                    (Keyword "yang-version" Number)
                    (Keyword "namespace" String)
                    (Keyword "description" String)
                    (Keyword "revision" Date
                      (comment)
                      (Keyword "description" String))
                    (INVALID "number" Number)))
                "#
            ),
            tree.to_string()
        );
    }
}
