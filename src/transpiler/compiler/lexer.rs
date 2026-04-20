use logos::Logos;

use crate::transpiler::compiler::ast::Span;
use crate::transpiler::error::TranspileError;

#[derive(Logos, Clone, Debug, Eq, PartialEq)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum TokenKind {
    #[token("(")]
    OpenParen,
    #[token(")")]
    CloseParen,
    #[token("[")]
    OpenBracket,
    #[token("]")]
    CloseBracket,
    #[token("{")]
    OpenBrace,
    #[token("}")]
    CloseBrace,
    #[token(",")]
    Comma,
    #[token(";")]
    Semi,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token("=")]
    Eq,
    #[token("|")]
    Pipe,
    #[token("&")]
    Ampersand,
    #[token("!")]
    Bang,
    #[token("=>")]
    FatArrow,
    #[token("->")]
    ThinArrow,
    #[regex(r#"\"([^\\\"]|\\.)*\""#)]
    String,
    #[regex(r#"'([^'\\]|\\.)'"#)]
    Char,
    #[regex(r#"'[A-Za-z_][A-Za-z0-9_]*"#)]
    Lifetime,
    #[regex(r"[0-9][A-Za-z0-9_\.]*")]
    Number,
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*")]
    Ident,
    #[regex(r"//[^\n]*", allow_greedy = true)]
    LineComment,
    #[regex(r"/\*([^*]|\*[^/])*\*/")]
    BlockComment,
    #[regex(r#"[^\sA-Za-z0-9_"'()\[\]{}.,;:=|&!]+"#)]
    Punct,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub text: String,
    pub span: Span,
}

pub fn lex(source: &str) -> Result<Vec<Token>, TranspileError> {
    let mut lexer = TokenKind::lexer(source);
    let mut tokens = Vec::new();

    while let Some(kind) = lexer.next() {
        match kind {
            Ok(TokenKind::LineComment | TokenKind::BlockComment) => {}
            Ok(kind) => tokens.push(Token {
                kind,
                text: lexer.slice().to_string(),
                span: lexer.span(),
            }),
            Err(_) => {
                let span = lexer.span();
                return Err(TranspileError::diagnostic(
                    format!("unexpected token `{}`", &source[span.clone()]),
                    span,
                ));
            }
        }
    }

    Ok(tokens)
}
