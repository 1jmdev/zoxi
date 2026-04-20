use std::ops::Range;

use crate::transpiler::compiler::lexer::Token;

pub type Span = Range<usize>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Delimiter {
    Paren,
    Bracket,
    Brace,
}

impl Delimiter {
    pub fn open(self) -> char {
        match self {
            Self::Paren => '(',
            Self::Bracket => '[',
            Self::Brace => '{',
        }
    }

}

#[derive(Clone, Debug)]
pub struct Module {
    pub nodes: Vec<SyntaxNode>,
}

#[derive(Clone, Debug)]
pub enum SyntaxNode {
    Token(Token),
    Group(Group),
}

#[derive(Clone, Debug)]
pub struct Group {
    pub delimiter: Delimiter,
    pub nodes: Vec<SyntaxNode>,
}

#[derive(Clone, Debug)]
pub struct Analysis {
    pub module: Module,
}
