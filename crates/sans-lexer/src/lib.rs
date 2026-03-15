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

            // Numeric literals (integer or float)
            '0'..='9' => {
                while pos < len && (bytes[pos] as char).is_ascii_digit() {
                    pos += 1;
                }
                // Check for decimal point (float literal)
                if pos < len && bytes[pos] == b'.' && pos + 1 < len && (bytes[pos + 1] as char).is_ascii_digit() {
                    pos += 1; // consume '.'
                    while pos < len && (bytes[pos] as char).is_ascii_digit() {
                        pos += 1;
                    }
                    let text = &source[start..pos];
                    let value: f64 = text.parse().map_err(|_| LexError {
                        message: format!("float literal '{}' is invalid", text),
                        span: start..pos,
                    })?;
                    tokens.push(Token {
                        kind: TokenKind::FloatLiteral(value),
                        span: start..pos,
                    });
                } else {
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
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    "while" => TokenKind::While,
                    "return" => TokenKind::Return,
                    "mut" => TokenKind::Mut,
                    "struct" => TokenKind::Struct,
                    "enum" => TokenKind::Enum,
                    "match" => TokenKind::Match,
                    "trait" => TokenKind::Trait,
                    "impl" => TokenKind::Impl,
                    "for" => TokenKind::For,
                    "self" => TokenKind::SelfValue,
                    "Self" => TokenKind::SelfType,
                    "spawn" => TokenKind::Spawn,
                    "channel" => TokenKind::Channel,
                    "mutex" => TokenKind::Mutex,
                    "array" => TokenKind::Array,
                    "in" => TokenKind::In,
                    "import" => TokenKind::Import,
                    "g" => TokenKind::Global,
                    "break" => TokenKind::Break,
                    "continue" => TokenKind::Continue,
                    _ => TokenKind::Identifier(text.to_string()),
                };
                tokens.push(Token {
                    kind,
                    span: start..pos,
                });
            }

            // Single-character tokens
            '+' => {
                if pos + 1 < len && bytes[pos + 1] == b'=' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::PlusEq, span: start..pos });
                } else {
                    pos += 1;
                    tokens.push(Token { kind: TokenKind::Plus, span: start..pos });
                }
            }
            '-' => {
                if pos + 1 < len && bytes[pos + 1] == b'=' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::MinusEq, span: start..pos });
                } else {
                    pos += 1;
                    tokens.push(Token { kind: TokenKind::Minus, span: start..pos });
                }
            }
            '*' => {
                if pos + 1 < len && bytes[pos + 1] == b'=' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::StarEq, span: start..pos });
                } else {
                    pos += 1;
                    tokens.push(Token { kind: TokenKind::Star, span: start..pos });
                }
            }
            '/' => {
                // Not a line comment (handled above), so it's a slash operator
                if pos + 1 < len && bytes[pos + 1] == b'=' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::SlashEq, span: start..pos });
                } else {
                    pos += 1;
                    tokens.push(Token { kind: TokenKind::Slash, span: start..pos });
                }
            }
            '%' => {
                if pos + 1 < len && bytes[pos + 1] == b'=' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::PercentEq, span: start..pos });
                } else {
                    pos += 1;
                    tokens.push(Token { kind: TokenKind::Percent, span: start..pos });
                }
            }
            '=' => {
                if pos + 1 < len && bytes[pos + 1] == b'=' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::EqEq, span: start..pos });
                } else if pos + 1 < len && bytes[pos + 1] == b'>' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::FatArrow, span: start..pos });
                } else {
                    pos += 1;
                    tokens.push(Token { kind: TokenKind::Eq, span: start..pos });
                }
            }
            '!' => {
                if pos + 1 < len && bytes[pos + 1] == b'=' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::NotEq, span: start..pos });
                } else {
                    pos += 1;
                    tokens.push(Token { kind: TokenKind::Bang, span: start..pos });
                }
            }
            '<' => {
                if pos + 1 < len && bytes[pos + 1] == b'=' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::LtEq, span: start..pos });
                } else {
                    pos += 1;
                    tokens.push(Token { kind: TokenKind::Lt, span: start..pos });
                }
            }
            '>' => {
                if pos + 1 < len && bytes[pos + 1] == b'=' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::GtEq, span: start..pos });
                } else {
                    pos += 1;
                    tokens.push(Token { kind: TokenKind::Gt, span: start..pos });
                }
            }
            '&' => {
                if pos + 1 < len && bytes[pos + 1] == b'&' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::And, span: start..pos });
                } else {
                    return Err(LexError {
                        message: "unexpected character '&'".to_string(),
                        span: start..start + 1,
                    });
                }
            }
            '|' => {
                if pos + 1 < len && bytes[pos + 1] == b'|' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::Or, span: start..pos });
                } else {
                    pos += 1;
                    tokens.push(Token { kind: TokenKind::Pipe, span: start..pos });
                }
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
            '[' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::LBracket, span: start..pos });
            }
            ']' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::RBracket, span: start..pos });
            }
            ',' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::Comma, span: start..pos });
            }
            ':' => {
                if pos + 1 < len && bytes[pos + 1] == b':' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::ColonColon, span: start..pos });
                } else if pos + 1 < len && bytes[pos + 1] == b'=' {
                    pos += 2;
                    tokens.push(Token { kind: TokenKind::ColonEq, span: start..pos });
                } else {
                    pos += 1;
                    tokens.push(Token { kind: TokenKind::Colon, span: start..pos });
                }
            }

            '.' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::Dot, span: start..pos });
            }

            '?' => {
                pos += 1;
                tokens.push(Token { kind: TokenKind::Question, span: start..pos });
            }

            '"' => {
                if pos + 2 < len && bytes[pos + 1] == b'"' && bytes[pos + 2] == b'"' {
                    // Triple-quoted multiline string
                    pos += 3; // skip opening """
                    let content_start = pos;
                    loop {
                        if pos + 2 >= len {
                            return Err(LexError {
                                message: "unterminated multiline string".to_string(),
                                span: start..pos,
                            });
                        }
                        if bytes[pos] == b'"' && bytes[pos + 1] == b'"' && bytes[pos + 2] == b'"' {
                            break;
                        }
                        pos += 1;
                    }
                    let value = source[content_start..pos].to_string();
                    pos += 3; // skip closing """
                    tokens.push(Token {
                        kind: TokenKind::StringLiteral(value),
                        span: start..pos,
                    });
                } else {
                    // Regular string with interpolation support
                    pos += 1; // skip opening quote
                    let mut parts: Vec<token::StringPart> = Vec::new();
                    let mut has_interp = false;
                    let mut cur = String::new();
                    while pos < len && bytes[pos] != b'"' {
                        if bytes[pos] == b'\\' {
                            if pos + 1 < len {
                                match bytes[pos + 1] {
                                    b'n' => { cur.push('\n'); pos += 2; }
                                    b't' => { cur.push('\t'); pos += 2; }
                                    b'\\' => { cur.push('\\'); pos += 2; }
                                    b'"' => { cur.push('"'); pos += 2; }
                                    b'{' => { cur.push('{'); pos += 2; }
                                    _ => { cur.push('\\'); pos += 1; }
                                }
                            } else {
                                cur.push('\\');
                                pos += 1;
                            }
                        } else if bytes[pos] == b'{' {
                            // Scan for matching closing brace, handling nesting
                            // Stop at '"' or '\' to avoid crossing string escape boundaries
                            let mut probe = pos + 1;
                            let mut depth = 1;
                            while probe < len && depth > 0 {
                                if bytes[probe] == b'"' || bytes[probe] == b'\\' { break; }
                                if bytes[probe] == b'{' { depth += 1; }
                                if bytes[probe] == b'}' { depth -= 1; }
                                if depth > 0 { probe += 1; }
                            }
                            if probe < len && depth == 0 {
                                let content = source[pos + 1..probe].trim().to_string();
                                if !content.is_empty() {
                                    has_interp = true;
                                    if !cur.is_empty() {
                                        parts.push(token::StringPart::Literal(std::mem::take(&mut cur)));
                                    }
                                    // Check if it's a simple identifier
                                    let is_ident = content.bytes().next().map_or(false, |b| b.is_ascii_alphabetic() || b == b'_')
                                        && content.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
                                    if is_ident {
                                        parts.push(token::StringPart::Ident(content));
                                    } else {
                                        parts.push(token::StringPart::Expr(content));
                                    }
                                    pos = probe + 1; // skip past closing }
                                } else {
                                    cur.push('{');
                                    pos += 1;
                                }
                            } else {
                                cur.push('{');
                                pos += 1;
                            }
                        } else {
                            cur.push(bytes[pos] as char);
                            pos += 1;
                        }
                    }
                    if pos >= len {
                        return Err(LexError {
                            message: "unterminated string literal".to_string(),
                            span: start..pos,
                        });
                    }
                    pos += 1; // skip closing quote
                    if has_interp {
                        if !cur.is_empty() {
                            parts.push(token::StringPart::Literal(cur));
                        }
                        tokens.push(Token {
                            kind: TokenKind::InterpolatedString(parts),
                            span: start..pos,
                        });
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::StringLiteral(cur),
                            span: start..pos,
                        });
                    }
                }
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
    fn lex_bool_keywords() {
        let tokens = lex("true false").unwrap();
        assert_eq!(kinds(&tokens), vec![True, False, Eof]);
    }

    #[test]
    fn lex_comparison_operators() {
        let tokens = lex("== != < > <= >=").unwrap();
        assert_eq!(kinds(&tokens), vec![EqEq, NotEq, Lt, Gt, LtEq, GtEq, Eof]);
    }

    #[test]
    fn lex_boolean_operators() {
        let tokens = lex("&& || !").unwrap();
        assert_eq!(kinds(&tokens), vec![And, Or, Bang, Eof]);
    }

    #[test]
    fn lex_if_else_keywords() {
        let tokens = lex("if else").unwrap();
        assert_eq!(kinds(&tokens), vec![If, Else, Eof]);
    }

    #[test]
    fn lex_eq_vs_eqeq() {
        let tokens = lex("= ==").unwrap();
        assert_eq!(kinds(&tokens), vec![Eq, EqEq, Eof]);
    }

    #[test]
    fn lex_while_return_mut_keywords() {
        let tokens = lex("while return mut").unwrap();
        assert_eq!(kinds(&tokens), vec![While, Return, Mut, Eof]);
    }

    #[test]
    fn lex_string_literal() {
        let tokens = lex(r#""hello""#).unwrap();
        assert_eq!(kinds(&tokens), vec![StringLiteral("hello".to_string()), Eof]);
    }

    #[test]
    fn lex_struct_keyword() {
        let tokens = lex("struct").unwrap();
        assert_eq!(kinds(&tokens), vec![Struct, Eof]);
    }

    #[test]
    fn lex_dot_token() {
        let tokens = lex("a.b").unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![Identifier("a".to_string()), Dot, Identifier("b".to_string()), Eof]
        );
    }

    #[test]
    fn lex_enum_match_keywords() {
        let tokens = lex("enum match").unwrap();
        assert_eq!(kinds(&tokens), vec![Enum, Match, Eof]);
    }

    #[test]
    fn lex_fat_arrow() {
        let tokens = lex("=>").unwrap();
        assert_eq!(kinds(&tokens), vec![FatArrow, Eof]);
    }

    #[test]
    fn lex_colon_colon() {
        let tokens = lex("::").unwrap();
        assert_eq!(kinds(&tokens), vec![ColonColon, Eof]);
    }

    #[test]
    fn lex_trait_impl_for_keywords() {
        let tokens = lex("trait impl for").unwrap();
        assert_eq!(kinds(&tokens), vec![Trait, Impl, For, Eof]);
    }

    #[test]
    fn lex_self_keywords() {
        let tokens = lex("self Self").unwrap();
        assert_eq!(kinds(&tokens), vec![SelfValue, SelfType, Eof]);
    }

    #[test]
    fn lex_spawn_keyword() {
        let tokens = lex("spawn").unwrap();
        assert_eq!(kinds(&tokens), vec![Spawn, Eof]);
    }

    #[test]
    fn lex_channel_keyword() {
        let tokens = lex("channel").unwrap();
        assert_eq!(kinds(&tokens), vec![Channel, Eof]);
    }

    #[test]
    fn lex_mutex_keyword() {
        let tokens = lex("mutex").unwrap();
        assert_eq!(kinds(&tokens), vec![Mutex, Eof]);
    }

    #[test]
    fn lex_array_keyword() {
        let tokens = lex("array").unwrap();
        assert_eq!(kinds(&tokens), vec![Array, Eof]);
    }

    #[test]
    fn lex_in_keyword() {
        let tokens = lex("in").unwrap();
        assert_eq!(kinds(&tokens), vec![In, Eof]);
    }

    #[test]
    fn lex_records_spans() {
        let tokens = lex("fn main").unwrap();
        // "fn" is at bytes 0..2, "main" at bytes 3..7
        assert_eq!(tokens[0].span, 0..2);
        assert_eq!(tokens[1].span, 3..7);
    }

    #[test]
    fn lex_import_keyword() {
        let tokens = lex("import \"utils\"").unwrap();
        assert_eq!(
            kinds(&tokens),
            vec![Import, StringLiteral("utils".to_string()), Eof]
        );
    }

    #[test]
    fn lex_pipe_token() {
        let tokens = lex("|x:I| I { x }").unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Pipe);
        // Second pipe
        assert!(tokens.iter().filter(|t| t.kind == TokenKind::Pipe).count() == 2);
    }
}
