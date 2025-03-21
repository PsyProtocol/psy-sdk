use crate::{Error, Spanned, Token};

pub struct GenericTokenTransformer<'input, I>
where
    I: Iterator<Item = Spanned<Token<'input>>>,
{
    underlying: I,
    in_generics: i32,
    buffered: Option<Spanned<Token<'input>>>,
}

impl<'input, I> GenericTokenTransformer<'input, I>
where
    I: Iterator<Item = Spanned<Token<'input>>>,
{
    pub fn new(lexer: I) -> Self {
        Self {
            underlying: lexer,
            in_generics: 0,
            buffered: None,
        }
    }
}

impl<'input, I> Iterator for GenericTokenTransformer<'input, I>
where
    I: Iterator<Item = Spanned<Token<'input>>>,
{
    type Item = Spanned<Token<'input>>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(buffered) = self.buffered.take() {
            return Some(buffered);
        }

        let next = self.underlying.next()?;
        match next {
            Ok((start, Token::OperatorLt, end)) => {
                self.in_generics += 1;
                Some(Ok((start, Token::OperatorLt, end)))
            }
            Ok((start, Token::OperatorGt, end)) => {
                self.in_generics -= 1;
                if self.in_generics < 0 {
                    self.in_generics = 0;
                }
                Some(Ok((start, Token::OperatorGt, end)))
            }
            Ok((start, Token::OperatorShr, end)) => {
                if self.in_generics > 0 {
                    let mid = start + 1;
                    self.buffered = Some(Ok((mid, Token::OperatorGt, end)));
                    Some(Ok((start, Token::OperatorGt, mid)))
                } else {
                    Some(Ok((start, Token::OperatorShr, end)))
                }
            }
            Ok((start, token, end)) => Some(Ok((start, token, end))),
            Err(e) => Some(Err(e)),
        }
    }
}
