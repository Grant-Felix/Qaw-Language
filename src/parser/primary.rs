//! parse_primary — 字面量 / 标识符 / 括号表达式 / 数组字面量

use super::Parser;
use crate::ast::{new_array_lit, new_bool_lit, new_char_lit, new_float_lit, new_ident, new_int_lit, new_string_lit, Expr, ExprData, Kind};
use crate::lexer::TokKind;

impl Parser {
    /// 基本元素：字面量、标识符、括号表达式、数组字面量
    pub(super) fn parse_primary(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        let kind = self.current.kind;
        let lexeme = self.current.lexeme.clone();

        match kind {
            TokKind::IntLit => {
                let v: i64 = lexeme.parse().unwrap_or(0);
                self.advance();
                new_int_lit(v, line, col)
            }
            TokKind::FloatLit => {
                let v: f64 = lexeme.parse().unwrap_or(0.0);
                self.advance();
                new_float_lit(v, line, col)
            }
            TokKind::StringLit => {
                self.advance();
                new_string_lit(self.parse_string_interp(&lexeme, line, col), line, col)
            }
            TokKind::CharLit => {
                let v: i32 = lexeme.chars().next().map(|c| c as i32).unwrap_or(0);
                self.advance();
                new_char_lit(v, line, col)
            }
            TokKind::BoolLit if lexeme == "true" => {
                self.advance();
                new_bool_lit(true, line, col)
            }
            TokKind::BoolLit if lexeme == "false" => {
                self.advance();
                new_bool_lit(false, line, col)
            }
            TokKind::KwTrue => {
                self.advance();
                new_bool_lit(true, line, col)
            }
            TokKind::KwFalse => {
                self.advance();
                new_bool_lit(false, line, col)
            }
            TokKind::Ident => {
                self.advance();
                new_ident(lexeme, line, col)
            }
            TokKind::LParen => {
                self.advance();
                let e = self.parse_expr();
                self.expect(TokKind::RParen, "')'");
                e
            }
            TokKind::LBracket => {
                // 数组字面量：[a, b, c]（A5）
                self.advance(); // [
                let mut items = Vec::new();
                if !self.check(TokKind::RBracket) {
                    loop {
                        items.push(self.parse_expr());
                        if !self.match_tok(TokKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(TokKind::RBracket, "']'");
                new_array_lit(items, line, col)
            }
            _ => {
                self.error(&format!("期望表达式，得到 {:?}", kind));
                Expr {
                    kind: Kind::Error,
                    line,
                    col,
                    data: ExprData::IntLit(0),
                }
            }
        }
    }
}
