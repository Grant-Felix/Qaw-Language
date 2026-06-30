//! parse_if / parse_while / parse_for / parse_match — 控制流语句

use super::Parser;
use super::free_var;
use crate::ast::{match_arm, new_block, new_for_in, new_for_range, new_if, new_match, new_while, Expr};
use crate::lexer::TokKind;

impl Parser {
    /// 解析 if
    pub(super) fn parse_if(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        self.advance(); // if

        // 兼容 if cond {} 与 if (cond) {}
        let cond = if self.match_tok(TokKind::LParen) {
            let c = self.parse_expr();
            self.expect(TokKind::RParen, "')'");
            c
        } else {
            self.parse_expr()
        };

        let then_block = self.parse_block();
        let else_block = if self.match_tok(TokKind::KwElse) {
            if self.check(TokKind::KwIf) {
                Some(self.parse_if())
            } else {
                Some(self.parse_block())
            }
        } else {
            None
        };
        new_if(cond, then_block, else_block, line, col)
    }

    /// 解析 while
    pub(super) fn parse_while(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        self.advance(); // while

        let cond = if self.match_tok(TokKind::LParen) {
            let c = self.parse_expr();
            self.expect(TokKind::RParen, "')'");
            c
        } else {
            self.parse_expr()
        };

        let body = self.parse_block();
        new_while(cond, body, line, col)
    }

    /// 解析 for
    pub(super) fn parse_for(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        self.advance(); // for

        if !self.check(TokKind::Ident) {
            self.error("期望循环变量名");
            return new_block(vec![], line, col);
        }
        let var = self.current.lexeme.clone();
        self.advance();

        if self.match_tok(TokKind::KwIn) {
            // for x in expr { }
            let iter = self.parse_expr();
            let body = self.parse_block();
            new_for_in(var, iter, body, line, col)
        } else if self.match_tok(TokKind::KwFrom) {
            // for x from a to b [step s] { }
            let start = self.parse_expr();
            if !self.expect(TokKind::KwTo, "'to' 或 'downto'") {
                free_var();
                return new_block(vec![], line, col);
            }
            let end = self.parse_expr();
            let step = if self.match_tok(TokKind::KwStep) {
                Some(self.parse_expr())
            } else {
                None
            };
            let body = self.parse_block();
            new_for_range(var, start, end, step, body, line, col)
        } else {
            self.error("期望 'in' 或 'from'");
            new_block(vec![], line, col)
        }
    }

    /// 解析 match
    pub(super) fn parse_match(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        self.advance(); // match

        let scrutinee = self.parse_expr();
        if !self.expect(TokKind::LBrace, "'{'") {
            return new_block(vec![], line, col);
        }

        let mut arms = Vec::new();
        while !self.check(TokKind::RBrace) && !self.check(TokKind::Eof) && arms.len() < 32 {
            if self.panic_mode {
                self.synchronize();
            }

            // 解析 pattern：所有"字面量类"分支（IntLit/FloatLit/StringLit/Ident）逻辑一致，
// 统一抽成 lexeme 捕获；KwTrue/KwFalse 走字面量名。
            let pattern = if self.check(TokKind::KwTrue) {
                self.advance();
                "true".to_string()
            } else if self.check(TokKind::KwFalse) {
                self.advance();
                "false".to_string()
            } else if matches!(
                self.current.kind,
                TokKind::IntLit | TokKind::FloatLit | TokKind::StringLit | TokKind::Ident
            ) {
                let s = self.current.lexeme.clone();
                self.advance();
                s
            } else {
                "_".to_string()
            };

            if !self.expect(TokKind::FatArrow, "'=>'") {
                while !self.check(TokKind::RBrace) && !self.check(TokKind::Eof) {
                    self.advance();
                }
                break;
            }

            let body = self.parse_expr();
            arms.push(match_arm(pattern, body));
            self.match_tok(TokKind::Comma);
        }
        self.expect(TokKind::RBrace, "'}'");
        new_match(scrutinee, arms, line, col)
    }
}