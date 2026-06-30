//! parse_struct / parse_enum — 用户自定义聚合类型

use super::Parser;
use crate::ast::{field_decl, new_enum, new_struct, variant_decl, Expr};
use crate::lexer::TokKind;

impl Parser {
    /// 解析 struct
    pub(super) fn parse_struct(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        self.advance(); // struct

        let name = if self.check(TokKind::Ident) {
            let n = self.current.lexeme.clone();
            self.advance();
            n
        } else {
            self.error("期望 struct 名");
            "error".to_string()
        };

        let mut fields = Vec::new();
        if self.expect(TokKind::LBrace, "'{'") {
            while !self.check(TokKind::RBrace) && !self.check(TokKind::Eof) && fields.len() < 64 {
                if self.check(TokKind::Ident) {
                    let fname = self.current.lexeme.clone();
                    self.advance();
                    if !self.expect(TokKind::Colon, "':'") { break; }
                    let ftype = if self.check(TokKind::Ident) {
                        let t = self.current.lexeme.clone();
                        self.advance();
                        t
                    } else {
                        self.error("期望类型");
                        "?".to_string()
                    };
                    fields.push(field_decl(fname, ftype));
                    self.match_tok(TokKind::Comma);
                } else {
                    self.advance();
                }
            }
            self.expect(TokKind::RBrace, "'}'");
        }
        new_struct(name, fields, line, col)
    }

    /// 解析 enum
    pub(super) fn parse_enum(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        self.advance(); // enum

        let name = if self.check(TokKind::Ident) {
            let n = self.current.lexeme.clone();
            self.advance();
            n
        } else {
            self.error("期望 enum 名");
            "error".to_string()
        };

        let mut variants = Vec::new();
        if self.expect(TokKind::LBrace, "'{'") {
            while !self.check(TokKind::RBrace) && !self.check(TokKind::Eof) && variants.len() < 64 {
                if self.check(TokKind::Ident) {
                    let vname = self.current.lexeme.clone();
                    self.advance();
                    // 可选 payload
                    let payload = if self.match_tok(TokKind::LParen) {
                        let s = "...".to_string();
                        while !self.check(TokKind::RParen) && !self.check(TokKind::Eof) {
                            self.advance();
                        }
                        self.expect(TokKind::RParen, "')'");
                        Some(s)
                    } else if self.match_tok(TokKind::LBrace) {
                        let s = "{...}".to_string();
                        while !self.check(TokKind::RBrace) && !self.check(TokKind::Eof) {
                            self.advance();
                        }
                        self.expect(TokKind::RBrace, "'}'");
                        Some(s)
                    } else {
                        None
                    };
                    variants.push(variant_decl(vname, payload));
                    self.match_tok(TokKind::Comma);
                } else {
                    self.advance();
                }
            }
            self.expect(TokKind::RBrace, "'}'");
        }
        new_enum(name, variants, line, col)
    }
}