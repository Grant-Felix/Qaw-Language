//! parse_expr (Pratt) + parse_unary + parse_postfix — 表达式解析

use super::Parser;
use crate::ast::{
    new_assign, new_binary, new_call, new_field_access, new_index, new_slice, new_unary,
    BinOp, Expr, Kind, UnOp,
};
use crate::lexer::TokKind;

impl Parser {
    // ============ 表达式解析（Pratt）============

    pub(super) fn parse_expr(&mut self) -> Expr {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: i32) -> Expr {
        let mut lhs = self.parse_unary();
        if lhs.kind == Kind::Error {
            return lhs;
        }

        loop {
            let (op, l_bp, r_bp, is_assign) = match self.current.kind {
                TokKind::Eq => (BinOp::Add /* 占位 */, 0, 1, true), // 特殊处理
                TokKind::OrOr => (BinOp::Or, 1, 2, false),
                TokKind::AndAnd => (BinOp::And, 2, 3, false),
                TokKind::EqEq => (BinOp::Eq, 3, 4, false),
                TokKind::NotEq => (BinOp::Neq, 3, 4, false),
                TokKind::Lt => (BinOp::Lt, 3, 4, false),
                TokKind::Le => (BinOp::Le, 3, 4, false),
                TokKind::Gt => (BinOp::Gt, 3, 4, false),
                TokKind::Ge => (BinOp::Ge, 3, 4, false),
                TokKind::Pipe => (BinOp::BitOr, 4, 5, false),
                TokKind::Caret => (BinOp::BitXor, 5, 6, false),
                TokKind::Amp => (BinOp::BitAnd, 6, 7, false),
                TokKind::Shl => (BinOp::Shl, 7, 8, false),
                TokKind::Shr => (BinOp::Shr, 7, 8, false),
                TokKind::Plus => (BinOp::Add, 8, 9, false),
                TokKind::Minus => (BinOp::Sub, 8, 9, false),
                TokKind::Star => (BinOp::Mul, 9, 10, false),
                TokKind::Slash => (BinOp::Div, 9, 10, false),
                TokKind::Percent => (BinOp::Mod, 9, 10, false),
                _ => break,
            };

            if l_bp < min_bp {
                break;
            }

            let line = self.current.line;
            let col = self.current.col;
            self.advance();

            if is_assign {
                // 赋值：右结合
                let rhs = self.parse_expr_bp(r_bp);
                lhs = new_assign(lhs, rhs, line, col);
            } else {
                let rhs = self.parse_expr_bp(r_bp);
                lhs = new_binary(op, lhs, rhs, line, col);
            }
        }
        lhs
    }

    /// 一元
    fn parse_unary(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        let op = match self.current.kind {
            TokKind::Minus => Some(UnOp::Neg),
            TokKind::Bang => Some(UnOp::Not),
            // ~ 不在词法中，作为 ^ 后续处理
            _ => None,
        };
        if let Some(o) = op {
            self.advance();
            let operand = self.parse_unary();
            return new_unary(o, operand, line, col);
        }
        self.parse_postfix()
    }

    /// 后缀：调用、字段、索引、切片
    fn parse_postfix(&mut self) -> Expr {
        let mut expr = self.parse_primary();
        loop {
            match self.current.kind {
                TokKind::LParen => {
                    let line = self.current.line;
                    let col = self.current.col;
                    self.advance(); // (
                    let mut args = Vec::new();
                    if !self.check(TokKind::RParen) {
                        loop {
                            args.push(self.parse_expr());
                            if !self.match_tok(TokKind::Comma) {
                                break;
                            }
                        }
                    }
                    self.expect(TokKind::RParen, "')'");
                    expr = new_call(expr, args, line, col);
                }
                TokKind::Dot => {
                    let line = self.current.line;
                    let col = self.current.col;
                    self.advance(); // .
                    if !self.check(TokKind::Ident) {
                        self.error("期望字段名");
                        break;
                    }
                    let field = self.current.lexeme.clone();
                    self.advance();
                    expr = new_field_access(expr, field, line, col);
                }
                TokKind::LBracket => {
                    let line = self.current.line;
                    let col = self.current.col;
                    self.advance(); // [
                    let start = if !self.check(TokKind::Colon) && !self.check(TokKind::RBracket) {
                        Some(self.parse_expr())
                    } else {
                        None
                    };
                    let mut end = None;
                    let mut inclusive = false;
                    if self.match_tok(TokKind::Colon) {
                        if !self.check(TokKind::RBracket) && !self.check(TokKind::DotDotEq) {
                            end = Some(self.parse_expr());
                        }
                        if self.match_tok(TokKind::DotDotEq) {
                            inclusive = true;
                            if !self.check(TokKind::RBracket) {
                                end = Some(self.parse_expr());
                            }
                        }
                    } else if self.match_tok(TokKind::DotDot) {
                        inclusive = false;
                        if !self.check(TokKind::RBracket) {
                            end = Some(self.parse_expr());
                        }
                    } else if end.is_none() {
                        // 纯索引
                    }
                    self.expect(TokKind::RBracket, "']'");
                    if end.is_none() {
                        // 索引
                        if let Some(idx) = start {
                            expr = new_index(expr, idx, line, col);
                        }
                    } else {
                        expr = new_slice(expr, start, end, inclusive, line, col);
                    }
                }
                _ => break,
            }
        }
        expr
    }
}