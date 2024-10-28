use lalrpop_util::lalrpop_mod;

pub use qed_ast::*;
pub(crate) use qed_builder::*;
pub use qed_lexer::*;

pub use qed_lexer::error::Error as LexicalError;

pub use crate::arena::Arena;
pub type Loc = usize;
pub type ParseError<'input> = lalrpop_util::ParseError<Loc, Token<'input>, LexicalError>;

lalrpop_mod!(pub qed);

pub fn parse<'input, F: ContextFelt, Ctx: Context<F>>(
    src: &'input str,
    arena: &mut Arena<F>,
    ctx: &mut Ctx,
) -> Result<Vec<StmtNode>, ParseError<'input>> {
    let lexer = Lexer::new(src);
    let errors = &mut Vec::new();
    Ok(qed::ProgramParser::new().parse(src, arena, ctx, errors, lexer)?)
}
