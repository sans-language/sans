pub mod token;

use token::{Span, Token, TokenKind};

/// An error produced during lexing.
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

/// Lex the given source string into a sequence of tokens.
///
/// The returned `Vec<Token>` always ends with a `TokenKind::Eof` token.
/// Returns `Err(LexError)` on the first unexpected character.
pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut pos = 0usize;

    while pos < len {
        let start = pos;
        let ch = bytes[pos] as char;

        match ch {
            // Skip whitespace
            ' ' | '\t' | '\r' | '\n' => {
                pos += 1;
            }

            // Line comments
            '/' if pos + 1 < len && bytes[pos + 1] == b'/' => {
                // Advance until newline or end of input
                while pos < len && bytes[pos] != b'\n' {
                    pos += 1;
                }
            }

            // Integer literals
            '0'..='9' => {
                while pos < len && (bytes[pos] as char).is_ascii_digit() {
                    pos += 1;
                }
                let text = &source[start..pos];
                let value: i64 = text.parse().map_err(|_| LexError {
                    message: format!("integer literal '{}' overflows i64", text),
                    span: start..pos,
                })?;
                tokens.push(Token {
                    kind: TokenKind::IntLiteral(value),
                    span: start..pos,
                });
            }

            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => {
                while pos < len
                    && ((bytes[pos] as char).is_alphanumeric() || bytes[pos] == b'_')
                {
                    pos += 1;
                }
                let text = &source[start..pos];
                let kind = match text {
                    "fn" => TokenKind::Fn,
                    "let" => TokenKind::Let,
                    _ => TokenKind::Identifier(text.to_string()),
                };
                tokens.push(Token {
                    kind,
                    span: start..pos,
                });
            }

            // Single-character tokens
            '+' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::Plus, span: start..pos });
            }
            '-' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::Minus, span: start..pos });
            }
            '*' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::Star, span: start..pos });
            }
            '/' => {
                // Not a line comment (handled above), so it's a slash operator
                pos += 1;
                tokens.push(Token { kind: TokenKind::Slash, span: start..pos });
            }
            '=' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::Eq, span: start..pos });
            }
            '(' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::LParen, span: start..pos });
            }
            ')' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::RParen, span: start..pos });
            }
            '{' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::LBrace, span: start..pos });
            }
            '}' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::RBrace, span: start..pos });
            }
            ',' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::Comma, span: start..pos });
            }
            ':' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::Colon, span: start..pos });
            }

            other => {
                return Err(LexError {
                    message: format!("unexpected character '{}'", other),
                    span: start..start + 1,
                });
            }
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        span: len..len,
    });

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use token::TokenKind::*;

    fn kinds(tokens: &[Token]) -> Vec<TokenKind> {
        tokens.iter().map(|t| t.kind.clone()).collect()
    }

    #[test]
    fn lex_integer_literal() {
        let tokens = lex("42").unwrap();
        assert_eq!(kinds(&tokens), vec![IntLiteral(42), Eof]);
    }

    #[test]
    fn lex_identifier() {
        let tokens = lex("foo").unwrap();
        assert_eq!(kinds(&tokens), vec![Identifier("foo".to_string()), Eof]);
    }

    #[test]
    fn lex_fn_keyword() {
        let tokens = lex("fn").unwrap();
        assert_eq!(kinds(&tokens), vec![Fn, Eof]);
    }

    #[test]
    fn lex_let_keyword() {
        let tokens = lex("let").unwrap();
        assert_eq!(kinds(&tokens), vec![Let, Eof]);
    }

    #[test]
    fn lex_operators() {
        let tokens = lex("+ - * /").unwrap();
        assert_eq!(kinds(&tokens), vec![Plus, Minus, Star, Slash, Eof]);
    }

    #[test]
    fn lex_delimiters_and_punctuation() {
        let tokens = lex("( ) { } , :").unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![LParen, RParen, LBrace, RBrace, Comma, Colon, Eof]
        );
    }

    #[test]
    fn lex_simple_function() {
        let tokens = lex("fn main() Int { 42 }").unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![
                Fn,
                Identifier("main".to_string()),
                LParen,
                RParen,
                Identifier("Int".to_string()),
                LBrace,
                IntLiteral(42),
                RBrace,
                Eof,
            ]
        );
    }

    #[test]
    fn lex_skips_whitespace_and_comments() {
        let tokens = lex("fn // comment\nmain").unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![Fn, Identifier("main".to_string()), Eof]
        );
    }

    #[test]
    fn lex_records_spans() {
        let tokens = lex("fn main").unwrap();
        // "fn" is at bytes 0..2, "main" at bytes 3..7
        assert_eq!(tokens[0].span, 0..2);
        assert_eq!(tokens[1].span, 3..7);
    }
}
