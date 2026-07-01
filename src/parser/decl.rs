//! parse_var_decl / parse_return / parse_expr_stmt — 声明类语句

use super::Parser;
use crate::ast::{new_expr_stmt, new_return, new_var_decl, type_nullable, Expr};
use crate::lexer::TokKind;

impl Parser {
    /// 解析变量声明
    pub(super) fn parse_var_decl(&mut self, is_mut: bool) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        self.advance(); // let / var / const

        let mut is_mut = is_mut;
        if self.match_tok(TokKind::KwMut) {
            is_mut = true;
        }

        let name = if self.check(TokKind::Ident) {
            let n = self.current.lexeme.clone();
            self.advance();
            n
        } else {
            self.error("期望变量名");
            "error".to_string()
        };

        // 类型注解（A1）：`: T` 或 `: T?`
        let type_annotation = if self.match_tok(TokKind::Colon) {
            match self.parse_type_annotation() {
                Some(mut ann) => {
                    if self.match_tok(TokKind::Question) {
                        ann = type_nullable(ann);
                    }
                    Some(ann)
                }
                None => None,
            }
        } else {
            None
        };

        let init = if self.match_tok(TokKind::Eq) {
            Some(self.parse_expr())
        } else {
            None
        };

        self.match_tok(TokKind::Semi);
        new_var_decl(name, type_annotation, is_mut, init, line, col)
    }

    /// 解析 return
    pub(super) fn parse_return(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        self.advance(); // return

        let value = if !self.check(TokKind::Semi) && !self.check(TokKind::RBrace) && !self.check(TokKind::Eof) {
            Some(self.parse_expr())
        } else {
            None
        };
        self.match_tok(TokKind::Semi);
        new_return(value, line, col)
    }

    /// 解析表达式语句
    pub(super) fn parse_expr_stmt(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        let expr = self.parse_expr();
        self.match_tok(TokKind::Semi);
        new_expr_stmt(expr, line, col)
    }
}