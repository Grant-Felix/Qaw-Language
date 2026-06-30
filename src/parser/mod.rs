//! Qaw 解析器
//!
//! 递归下降 + Pratt 运算符解析。
//! v0.5 完整版，包含控制流、match、函数、struct、enum、模式匹配。
//!
//! 模块拆分：
//! - `mod.rs`     本文件：Parser 结构 + 辅助方法 + 顶层入口 + 测试
//! - `function`   parse_function（含参数列表）
//! - `aggregate`  parse_struct / parse_enum
//! - `decl`       parse_var_decl / parse_return / parse_expr_stmt
//! - `control`    parse_if / parse_while / parse_for / parse_match
//! - `expr`       parse_expr (Pratt) + parse_unary + parse_postfix
//! - `primary`    parse_primary (字面量 / 标识符 / 括号)
//! - `interp`     parse_string_interp + parse_inline_expression

use crate::ast::*;
use crate::lexer::{Lexer, TokKind, Token};

/// 解析错误
#[derive(Debug, Clone)]
pub struct ParseError {
    pub line: u32,
    pub col: u32,
    pub message: String,
}

/// 解析器
pub struct Parser {
    pub(crate) lex: Lexer,
    pub(crate) current: Token,
    pub(crate) peeked: Option<Token>,
    pub(crate) panic_mode: bool,
    pub(crate) last_error: Option<ParseError>,
}

impl Parser {
    pub fn new(mut lex: Lexer) -> Self {
        let first = lex.next_token();
        Parser {
            lex,
            current: first,
            peeked: None,
            panic_mode: false,
            last_error: None,
        }
    }

    // ============ Token 流辅助（仅本模块可见，子模块通过 super::Parser 访问）============
    // 注：以下方法是 `private`（默认），但 Rust 允许子模块访问父模块的私有项。

    fn peek(&mut self) -> Token {
        if self.peeked.is_none() {
            let t = self.lex.next_token();
            self.peeked = Some(t);
        }
        self.peeked.as_ref().unwrap().clone()
    }

    pub(crate) fn advance(&mut self) {
        if let Some(t) = self.peeked.take() {
            self.current = t;
        } else {
            self.current = self.lex.next_token();
        }
    }

    pub(crate) fn check(&self, kind: TokKind) -> bool {
        self.current.kind == kind
    }

    pub(crate) fn match_tok(&mut self, kind: TokKind) -> bool {
        if self.current.kind == kind {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(crate) fn expect(&mut self, kind: TokKind, what: &str) -> bool {
        if self.current.kind == kind {
            self.advance();
            return true;
        }
        if !self.panic_mode {
            self.panic_mode = true;
            self.last_error = Some(ParseError {
                line: self.current.line,
                col: self.current.col,
                message: format!("期望 {}, 得到 {:?} at {}:{}",
                    what, self.current.kind, self.current.line, self.current.col),
            });
        }
        false
    }

    pub(crate) fn error(&mut self, msg: &str) {
        if !self.panic_mode {
            self.panic_mode = true;
            self.last_error = Some(ParseError {
                line: self.current.line,
                col: self.current.col,
                message: msg.to_string(),
            });
        }
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;
        while !self.check(TokKind::Eof) {
            match self.current.kind {
                TokKind::KwFunc | TokKind::KwStruct | TokKind::KwEnum
                | TokKind::KwVar | TokKind::KwLet | TokKind::KwConst
                | TokKind::KwIf | TokKind::KwWhile | TokKind::KwFor
                | TokKind::KwReturn => return,
                _ => { self.advance(); }
            }
        }
    }

    // ============ 顶层入口 ============

    /// 顶层入口
    pub fn parse_program(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        let mut items = Vec::new();
        while !self.check(TokKind::Eof) && items.len() < 256 {
            if self.panic_mode {
                self.synchronize();
            }
            if let Some(item) = self.parse_item() {
                items.push(item);
            }
        }
        new_program(items, line, col)
    }

    /// 顶层项
    fn parse_item(&mut self) -> Option<Expr> {
        match self.current.kind {
            TokKind::KwFunc => Some(self.parse_function()),
            TokKind::KwStruct => Some(self.parse_struct()),
            TokKind::KwEnum => Some(self.parse_enum()),
            _ => {
                self.error(&format!(
                    "期望顶层项（func/struct/enum），得到 {:?} at {}:{}",
                    self.current.kind, self.current.line, self.current.col
                ));
                self.synchronize();
                None
            }
        }
    }

    /// 解析块
    fn parse_block(&mut self) -> Expr {
        let line = self.current.line;
        let col = self.current.col;
        if !self.expect(TokKind::LBrace, "'{'") {
            return new_block(vec![], line, col);
        }

        let mut stmts = Vec::new();
        while !self.check(TokKind::RBrace) && !self.check(TokKind::Eof) && stmts.len() < 256 {
            if self.panic_mode {
                self.synchronize();
            }
            if let Some(s) = self.parse_stmt() {
                stmts.push(s);
            }
        }
        self.expect(TokKind::RBrace, "'}'");
        new_block(stmts, line, col)
    }

    /// 解析一条语句（dispatch）
    fn parse_stmt(&mut self) -> Option<Expr> {
        let line = self.current.line;
        let col = self.current.col;
        match self.current.kind {
            TokKind::KwLet => Some(self.parse_var_decl(false)),
            TokKind::KwVar => Some(self.parse_var_decl(true)),
            TokKind::KwConst => Some(self.parse_var_decl(false)),
            TokKind::KwReturn => Some(self.parse_return()),
            TokKind::KwBreak => {
                self.advance();
                self.match_tok(TokKind::Semi);
                Some(new_break(line, col))
            }
            TokKind::KwContinue => {
                self.advance();
                self.match_tok(TokKind::Semi);
                Some(new_continue(line, col))
            }
            TokKind::KwIf => Some(self.parse_if()),
            TokKind::KwWhile => Some(self.parse_while()),
            TokKind::KwFor => Some(self.parse_for()),
            TokKind::KwMatch => Some(self.parse_match()),
            _ => Some(self.parse_expr_stmt()),
        }
    }

    pub fn last_error(&self) -> Option<&ParseError> {
        self.last_error.as_ref()
    }
}

// 子模块
mod function;
mod aggregate;
mod decl;
mod control;
mod expr;
mod primary;
mod interp;

fn free_var() {}

// ============ 测试 ============

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(src: &str) -> Expr {
        let lex = Lexer::new(src);
        let mut p = Parser::new(lex);
        p.parse_program()
    }

    #[test]
    fn test_empty_program() {
        let p = parse("");
        assert_eq!(p.kind, Kind::Program);
    }

    #[test]
    fn test_function_no_params() {
        let p = parse("func foo() {}");
        if let ExprData::Program(prog) = &p.data {
            assert_eq!(prog.items.len(), 1);
        } else { panic!(); }
    }

    #[test]
    fn test_function_with_params() {
        let p = parse("func add(x: int, y: int) -> int { return x }");
        if let ExprData::Program(prog) = &p.data {
            assert_eq!(prog.items.len(), 1);
        } else { panic!(); }
    }

    #[test]
    fn test_function_with_return_type() {
        let p = parse("func bar() -> int { }");
        if let ExprData::Program(prog) = &p.data {
            assert_eq!(prog.items.len(), 1);
        } else { panic!(); }
    }

    #[test]
    fn test_struct() {
        let p = parse("struct Point { x: f64, y: f64 }");
        if let ExprData::Program(prog) = &p.data {
            assert_eq!(prog.items.len(), 1);
        } else { panic!(); }
    }

    #[test]
    fn test_enum() {
        let p = parse("enum Color { Red, Green, Blue }");
        if let ExprData::Program(prog) = &p.data {
            assert_eq!(prog.items.len(), 1);
        } else { panic!(); }
    }

    #[test]
    fn test_multiple_items() {
        let p = parse("struct A { x: int } enum B { X } func c() {}");
        if let ExprData::Program(prog) = &p.data {
            assert_eq!(prog.items.len(), 3);
        } else { panic!(); }
    }

    #[test]
    fn test_expr_int() {
        let p = parse("func f() { let x = 42 }");
        if let ExprData::Program(prog) = &p.data {
            if let ExprData::Function(fd) = &prog.items[0].data {
                if let ExprData::Block(b) = &fd.body.data {
                    if let ExprData::VarDecl(v) = &b.stmts[0].data {
                        if let ExprData::IntLit(n) = v.init.as_ref().unwrap().data {
                            assert_eq!(n, 42);
                        } else { panic!(); }
                    } else { panic!(); }
                } else { panic!(); }
            } else { panic!(); }
        } else { panic!(); }
    }

    #[test]
    fn test_expr_precedence() {
        // 1 + 2 * 3 应解析为 1 + (2 * 3)
        let p = parse("func f() { let x = 1 + 2 * 3 }");
        if let ExprData::Program(prog) = &p.data {
            if let ExprData::Function(fd) = &prog.items[0].data {
                if let ExprData::Block(b) = &fd.body.data {
                    if let ExprData::VarDecl(v) = &b.stmts[0].data {
                        if let ExprData::BinaryOp(bin) = &v.init.as_ref().unwrap().data {
                            assert_eq!(bin.op, BinOp::Add);
                        } else { panic!(); }
                    } else { panic!(); }
                } else { panic!(); }
            } else { panic!(); }
        } else { panic!(); }
    }

    #[test]
    fn test_match() {
        let p = parse("func f() { match x { 1 => 0 2 => 0 _ => 0 } }");
        if let ExprData::Program(prog) = &p.data {
            assert_eq!(prog.items.len(), 1);
        } else { panic!(); }
    }

    #[test]
    fn test_chinese_function() {
        let p = parse("func 中文函数() {}");
        if let ExprData::Program(prog) = &p.data {
            if let ExprData::Function(f) = &prog.items[0].data {
                assert_eq!(f.name, "中文函数");
            } else { panic!(); }
        } else { panic!(); }
    }
}