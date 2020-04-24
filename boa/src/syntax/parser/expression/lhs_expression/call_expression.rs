use super::arguments::Arguments;
use crate::{
    syntax::{
        ast::{node::Node, punc::Punctuator, token::TokenKind},
        parser::{
            expression::Expression, AllowAwait, AllowYield, Cursor, ParseError, ParseResult,
            TokenParser,
        },
    },
    Interner,
};

/// Parses a call expression.
///
/// More information:
///  - [ECMAScript specification][spec]
///
/// [spec]: https://tc39.es/ecma262/#prod-CallExpression
#[derive(Debug)]
pub(super) struct CallExpression {
    allow_yield: AllowYield,
    allow_await: AllowAwait,
    first_member_expr: Node,
}

impl CallExpression {
    /// Creates a new `CallExpression` parser.
    pub(super) fn new<Y, A>(allow_yield: Y, allow_await: A, first_member_expr: Node) -> Self
    where
        Y: Into<AllowYield>,
        A: Into<AllowAwait>,
    {
        Self {
            allow_yield: allow_yield.into(),
            allow_await: allow_await.into(),
            first_member_expr,
        }
    }
}

impl TokenParser for CallExpression {
    type Output = Node;

    fn parse(self, cursor: &mut Cursor<'_>, interner: &mut Interner) -> ParseResult {
        let mut lhs = if cursor
            .next_if_skip_lineterminator(TokenKind::Punctuator(Punctuator::OpenParen))
            .is_some()
        {
            let args =
                Arguments::new(self.allow_yield, self.allow_await).parse(cursor, interner)?;
            Node::call(self.first_member_expr, args)
        } else {
            let next_token = cursor
                .next_skip_lineterminator()
                .ok_or(ParseError::AbruptEnd)?;
            return Err(ParseError::expected(
                vec![Punctuator::OpenParen.to_string()],
                next_token.display(interner).to_string(),
                next_token.pos,
                "call expression",
            ));
        };

        while let Some(tok) = cursor.peek_skip_lineterminator() {
            match tok.kind {
                TokenKind::Punctuator(Punctuator::OpenParen) => {
                    let _ = cursor
                        .next_skip_lineterminator()
                        .ok_or(ParseError::AbruptEnd)?; // We move the cursor.
                    let args = Arguments::new(self.allow_yield, self.allow_await)
                        .parse(cursor, interner)?;
                    lhs = Node::call(lhs, args);
                }
                TokenKind::Punctuator(Punctuator::Dot) => {
                    let _ = cursor
                        .next_skip_lineterminator()
                        .ok_or(ParseError::AbruptEnd)?; // We move the cursor.
                    match &cursor
                        .next_skip_lineterminator()
                        .ok_or(ParseError::AbruptEnd)?
                        .kind
                    {
                        TokenKind::Identifier(name) => {
                            lhs = Node::get_const_field(lhs, *name);
                        }
                        TokenKind::Keyword(kw) => {
                            lhs =
                                Node::get_const_field(lhs, interner.get_or_intern(kw.to_string()));
                        }
                        _ => {
                            return Err(ParseError::expected(
                                vec![String::from("identifier")],
                                tok.display(interner).to_string(),
                                tok.pos,
                                "call expression",
                            ));
                        }
                    }
                }
                TokenKind::Punctuator(Punctuator::OpenBracket) => {
                    let _ = cursor
                        .next_skip_lineterminator()
                        .ok_or(ParseError::AbruptEnd)?; // We move the cursor.
                    let idx = Expression::new(true, self.allow_yield, self.allow_await)
                        .parse(cursor, interner)?;
                    cursor.expect(Punctuator::CloseBracket, "call expression", interner)?;
                    lhs = Node::get_field(lhs, idx);
                }
                _ => break,
            }
        }
        Ok(lhs)
    }
}
