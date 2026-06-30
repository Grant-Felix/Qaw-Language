//! parse_string_interp + parse_inline_expression — 字符串 ${...} 插值
//!
//! 词法层只把字符串原样切下来；插值在这里递归解析表达式。

use super::Parser;
use crate::ast::{Expr, InterpPart, StringLit};
use crate::lexer::{Lexer, Token};

impl Parser {
    /// 解析字符串字面量中的 ${...} 插值
    pub(super) fn parse_string_interp(&mut self, raw: &str, line: u32, col: u32) -> StringLit {
        let mut parts: Vec<InterpPart> = Vec::new();
        let bytes: Vec<char> = raw.chars().collect();
        let mut i = 0usize;
        let mut text_start = 0usize;

        while i < bytes.len() {
            if i + 1 < bytes.len() && bytes[i] == '$' && bytes[i + 1] == '{' {
                // 1. 追加 ${ 之前的文本
                if i > text_start {
                    let chunk: String = bytes[text_start..i].iter().collect();
                    parts.push(InterpPart::Text(chunk));
                }
                // 2. 找到匹配的 }
                let expr_start = i + 2;
                let mut depth = 1i32;
                let mut j = expr_start;
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 { break; }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                if j >= bytes.len() {
                    // 没找到 }，把剩下的都当文本
                    let chunk: String = bytes[expr_start..].iter().collect();
                    parts.push(InterpPart::Text(format!("${{{}}}", chunk)));
                    i = bytes.len();
                    text_start = i;
                    continue;
                }
                // 3. 取出 ${...} 内部表达式文本，递归解析
                let expr_src: String = bytes[expr_start..j].iter().collect();
                let inner_line = line;
                let inner_col = col + (i as u32) + 2;
                let inner = self.parse_inline_expression(&expr_src, inner_line, inner_col);
                parts.push(InterpPart::Expr(Box::new(inner)));
                i = j + 1;
                text_start = i;
            } else {
                i += 1;
            }
        }

        // 收尾
        if text_start < bytes.len() {
            let chunk: String = bytes[text_start..].iter().collect();
            parts.push(InterpPart::Text(chunk));
        }

        if parts.is_empty() {
            parts.push(InterpPart::Text(String::new()));
        }

        StringLit { parts }
    }

    /// 用临时 lexer 解析一小段表达式（用于 ${} 插值）
    fn parse_inline_expression(&mut self, src: &str, line: u32, col: u32) -> Expr {
        let tmp_lex = Lexer::new(src);
        let mut tmp_parser = Parser {
            lex: tmp_lex,
            current: Token::eof(line, col),
            peeked: None,
            panic_mode: false,
            last_error: None,
        };
        tmp_parser.current = tmp_parser.lex.next_token();
        tmp_parser.parse_expr()
    }
}

// 避免 unused 警告（保留以备未来扩展）
#[allow(dead_code)]
fn _unused(_: &Token) {}