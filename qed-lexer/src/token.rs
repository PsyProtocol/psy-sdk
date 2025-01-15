use crate::error::Error;
use logos::Logos;
use std::fmt;

#[derive(Logos, Clone, Debug, PartialEq)]
#[logos(skip r"[\s\t\r\n\f]+", skip r"//[^\n\r]*[\n\r]", skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/", error = Error)]
pub enum Token<'input> {
    #[token("const")]
    KeywordConst,
    #[token("let")]
    KeywordLet,

    #[token("mut")]
    KeywordMut,

    #[token("fn")]
    KeywordFn,
    #[token("struct")]
    KeywordStruct,
    #[token("enum")]
    KeywordEnum,
    #[token("impl")]
    KeywordImpl,
    #[token("trait")]
    KeywordTrait,
    #[token("return")]
    KeywordReturn,
    #[token("if")]
    KeywordIf,
    #[token("else")]
    KeywordElse,
    #[token("while")]
    KeywordWhile,
    #[token("for")]
    KeywordFor,

    #[token("as")]
    KeywordAs,

    #[token("new")]
    KeywordNew,

    #[token("mod")]
    KeywordMod,
    #[token("use")]
    KeywordUse,
    #[token("self")]
    KeywordSelf,
    #[token("crate")]
    KeywordCrate,
    #[token("super")]
    KeywordSuper,

    #[token("pub")]
    KeywordPub,

    #[token("std")]
    KeywordStd,

    #[token("bool")]
    TypeBool,
    #[token("Felt")]
    TypeFelt,
    #[token("Self")]
    TypeSelf,

    #[regex("[_a-zA-Z][_0-9a-zA-Z]*", |lex| lex.slice())]
    Ident(&'input str),

    #[regex(r"(?:0|[1-9]\d*)", |lex| lex.slice().parse())]
    Number(u64),
    #[token("false", |_| false)]
    #[token("true", |_| true)]
    Bool(bool),

    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("=")]
    Assign,
    #[token(";")]
    Semicolon,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token(":")]
    Colon,
    #[token("::")]
    DoubleColon,
    #[token("->")]
    Arrow,

    #[token("+")]
    OperatorAdd,
    #[token("-")]
    OperatorSub,
    #[token("*")]
    OperatorMul,
    #[token("/")]
    OperatorDiv,
    #[token("%")]
    OperatorMod,

    #[token("==")]
    OperatorEq,
    #[token("!=")]
    OperatorNeq,
    #[token("<")]
    OperatorLt,
    #[token("<=")]
    OperatorLte,
    #[token(">")]
    OperatorGt,
    #[token(">=")]
    OperatorGte,
    #[token("&&")]
    OperatorAnd,
    #[token("||")]
    OperatorOr,

    #[token("&")]
    OperatorBitAnd,
    #[token("|")]
    OperatorBitOr,
    #[token("^")]
    OperatorBitXor,

    #[token("<<")]
    OperatorShl,
    #[token(">>")]
    OperatorShr,

    #[token("!")]
    OperatorNot,
    #[token("+=")]
    OperatorAddAssign,
    #[token("-=")]
    OperatorSubAssign,
    #[token("*=")]
    OperatorMulAssign,
    #[token("/=")]
    OperatorDivAssign,
    #[token("%=")]
    OperatorModAssign,

    #[token("|=")]
    OperatorBitOrAssign,
    #[token("&=")]
    OperatorBitAndAssign,
    #[token("^=")]
    OperatorBitXorAssign,

    #[token("<<=")]
    OperatorBitShlAssign,

    #[token(">>=")]
    OperatorBitShrAssign,
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[test]
fn test_lex_comment() {
    let mut lex = Token::lexer("// This is a line comment\nlet x = 5;");
    assert_eq!(lex.next(), Some(Ok(Token::KeywordLet)));
    assert_eq!(lex.next(), Some(Ok(Token::Ident("x"))));
    assert_eq!(lex.next(), Some(Ok(Token::Assign)));
    assert_eq!(lex.next(), Some(Ok(Token::Number(5))));
    assert_eq!(lex.next(), Some(Ok(Token::Semicolon)));
    assert_eq!(lex.next(), None);

    let mut lex = Token::lexer("/* This is a\nblock comment */\nlet y = 10;");
    assert_eq!(lex.next(), Some(Ok(Token::KeywordLet)));
    assert_eq!(lex.next(), Some(Ok(Token::Ident("y"))));
    assert_eq!(lex.next(), Some(Ok(Token::Assign)));
    assert_eq!(lex.next(), Some(Ok(Token::Number(10))));
    assert_eq!(lex.next(), Some(Ok(Token::Semicolon)));
    assert_eq!(lex.next(), None);
}

#[test]
fn test_lex_integer() {
    let mut lex = Token::lexer("42 0 1234567890");
    assert_eq!(lex.next(), Some(Ok(Token::Number(42))));
    assert_eq!(lex.next(), Some(Ok(Token::Number(0))));
    assert_eq!(lex.next(), Some(Ok(Token::Number(1234567890))));
    assert_eq!(lex.next(), None);
}
