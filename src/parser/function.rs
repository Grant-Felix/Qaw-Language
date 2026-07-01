//! parse_function — 函数定义与参数列表
//!
//! 调用其他子模块的方法通过 `self.parse_xxx()`，因为它们都在 `super::Parser` 上。
//! 辅助方法（advance/match_tok/...）通过 super::Parser 的继承可见性访问。

use super::Parser;
use crate::ast::{new_function, param, type_nullable, Expr};
use crate::lexer::TokKind;

impl Parser {
    /// 解析函数
    pub(super) fn parse_function(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        self.advance(); // func

        let name = if self.check(TokKind::Ident) {
            let n = self.current.lexeme.clone();
            self.advance();
            n
        } else {
            self.error("期望函数名");
            "error".to_string()
        };

        // 参数列表
        let mut params = Vec::new();
        if self.match_tok(TokKind::LParen) {
            if !self.check(TokKind::RParen) {
                loop {
                    if self.check(TokKind::Ident) {
                        let pname = self.current.lexeme.clone();
                        self.advance();
                        // 参数类型注解（A1）：`: T` 或 `: T?`
                        let ptype = if self.match_tok(TokKind::Colon) {
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
                        params.push(param(pname, ptype));
                    } else {
                        self.error("期望参数名");
                        break;
                    }
                    if !self.match_tok(TokKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(TokKind::RParen, "')'");
        }

        // 返回类型（v0.20：第一版暂保持 Option<String>，T? 形式待 v0.30 一起做）
        let ret_type = if self.match_tok(TokKind::Arrow) {
            if self.check(TokKind::Ident) {
                let t = self.current.lexeme.clone();
                self.advance();
                Some(t)
            } else {
                self.error("期望返回类型");
                None
            }
        } else {
            None
        };

        // 函数体
        let body = self.parse_block();
        new_function(name, params, ret_type, body, line, col)
    }
}