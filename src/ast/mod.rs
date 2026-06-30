//! Qaw AST 节点定义
//!
//! Tagged union 设计：所有节点共享 `Expr` 结构体，
//! 通过 `kind` 字段判断类型，union 提供类型化访问。
//!
//! 模块拆分：
//! - `mod.rs`     本文件：核心类型 Kind / Expr / ExprData + Display + 调试打印 + 测试
//! - `literal`    StringLit / InterpPart + 字面量构造器
//! - `expr`       BinOp / UnOp / BinaryOp / UnaryOp / Call / FieldAccess / Index / Slice + 表达式构造器
//! - `stmt`       VarDecl / Assign / ExprStmt / Block / IfStmt / WhileStmt / ForKind / ForStmt / ReturnStmt / MatchArm / MatchStmt + 语句构造器
//! - `decl`       Param / Function / FieldDecl / StructDecl / VariantDecl / EnumDecl / Program + 声明构造器
//!
//! 警告抑制：AST 故意"超规格"（含未来 v0.5/v0.9 才需要的字段与构造器），
//! 当前 parser / interpreter 尚未全部使用。#[allow(dead_code)] 避免误报。

#![allow(dead_code)]

use std::fmt;

// re-export 子模块的公开 API（用于 `crate::ast::new_xxx` 等扁平调用面）
// #[allow(unused_imports)]：本 crate 是 binary crate，无外部用户，
// re-export 仅服务内部 `super::*` 调用与未来可能的 library 化。
#[allow(unused_imports)]
pub use literal::{interp_expr, interp_text, new_array_lit, new_bool_lit, new_char_lit, new_float_lit, new_ident, new_int_lit, new_string_lit, string_lit_simple, InterpPart, StringLit};
#[allow(unused_imports)]
pub use expr::{
    new_binary, new_call, new_field_access, new_index, new_slice, new_unary, BinOp, BinaryOp,
    Call, FieldAccess, Index, Slice, UnOp, UnaryOp,
};
#[allow(unused_imports)]
pub use stmt::{
    new_assign, new_block, new_break, new_continue, new_defer, new_expr_stmt, new_for_in,
    new_for_range, new_if, new_match, new_return, new_var_decl, new_while, Assign, Block,
    DeferStmt, ExprStmt, ForKind, ForStmt, IfStmt, MatchArm, MatchStmt, ReturnStmt, VarDecl,
    WhileStmt,
};
#[allow(unused_imports)]
pub use decl::{
    field_decl, match_arm, new_enum, new_function, new_program, new_struct, param, variant_decl,
    EnumDecl, FieldDecl, Function, Param, Program, StructDecl, VariantDecl,
};

/// 节点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    // 字面量
    IntLit,
    FloatLit,
    StringLit,
    BoolLit,
    CharLit,
    ArrayLit,
    Ident,
    // 表达式
    BinaryOp,
    UnaryOp,
    Call,
    FieldAccess,
    Index,
    Slice,
    // 语句
    VarDecl,
    Assign,
    ExprStmt,
    Block,
    IfStmt,
    WhileStmt,
    ForStmt,
    ReturnStmt,
    BreakStmt,
    ContinueStmt,
    MatchStmt,
    DeferStmt,
    // 声明
    Function,
    StructDecl,
    EnumDecl,
    // 顶层
    Program,
    // 错误占位
    Error,
}

/// 节点主体（tagged union）
#[derive(Debug, Clone)]
pub enum ExprData {
    IntLit(i64),
    FloatLit(f64),
    StringLit(StringLit),
    BoolLit(bool),
    CharLit(i32),
    ArrayLit(Vec<Expr>),
    Ident(String),
    BinaryOp(BinaryOp),
    UnaryOp(UnaryOp),
    Call(Call),
    FieldAccess(FieldAccess),
    Index(Index),
    Slice(Slice),
    VarDecl(VarDecl),
    Assign(Assign),
    ExprStmt(ExprStmt),
    Block(Block),
    IfStmt(IfStmt),
    WhileStmt(WhileStmt),
    ForStmt(ForStmt),
    ReturnStmt(ReturnStmt),
    BreakStmt,
    ContinueStmt,
    MatchStmt(MatchStmt),
    DeferStmt(DeferStmt),
    Function(Function),
    StructDecl(StructDecl),
    EnumDecl(EnumDecl),
    Program(Program),
    Error,
}

/// 表达式节点
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: Kind,
    pub line: u32,
    pub col: u32,
    pub data: ExprData,
}

impl Expr {
    /// 构造一个叶子节点
    pub fn leaf(kind: Kind, line: u32, col: u32) -> Self {
        // 占位，会被立即覆盖
        Expr {
            kind,
            line,
            col,
            data: ExprData::IntLit(0),
        }
    }
}

impl fmt::Display for BinOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Eq => "==",
            BinOp::Neq => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
            BinOp::BitAnd => "&",
            BinOp::BitOr => "|",
            BinOp::BitXor => "^",
            BinOp::Shl => "<<",
            BinOp::Shr => ">>",
            BinOp::Concat => "++",
        })
    }
}

impl fmt::Display for UnOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            UnOp::Neg => "-",
            UnOp::Not => "!",
            UnOp::BitNot => "~",
        })
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}@{}:{}", self.kind, self.line, self.col)
    }
}

// ============ 子模块 ============

mod literal;
mod expr;
mod stmt;
mod decl;

// ============ 调试打印 ============

pub fn print_expr(node: &Expr, indent: usize) {
    let pad = " ".repeat(indent);
    print!("{}{:?}@{}:{}", pad, node.kind, node.line, node.col);
    match &node.data {
        ExprData::IntLit(v) => println!(" = {}", v),
        ExprData::FloatLit(v) => println!(" = {}", v),
        ExprData::BoolLit(v) => println!(" = {}", v),
        ExprData::CharLit(v) => println!(" = U+{:04X}", v),
        ExprData::ArrayLit(items) => {
            println!(" items={}", items.len());
            for it in items { print_expr(it, indent + 2); }
        }
        ExprData::Ident(s) => println!(" name={}", s),
        ExprData::StringLit(s) => {
            println!(" parts={}", s.parts.len());
            for p in &s.parts {
                match p {
                    InterpPart::Text(t) => println!("{}  text: \"{}\"", pad, t),
                    InterpPart::Expr(e) => print_expr(e, indent + 2),
                }
            }
        }
        ExprData::BinaryOp(b) => {
            println!(" op={}", b.op);
            print_expr(&b.lhs, indent + 2);
            print_expr(&b.rhs, indent + 2);
        }
        ExprData::UnaryOp(u) => {
            println!(" op={}", u.op);
            print_expr(&u.operand, indent + 2);
        }
        ExprData::Call(c) => {
            println!(" args={}", c.args.len());
            print_expr(&c.func, indent + 2);
            for a in &c.args {
                print_expr(a, indent + 2);
            }
        }
        ExprData::FieldAccess(f) => {
            println!("{} field={}", pad, f.field);
            print_expr(&f.obj, indent + 2);
        }
        ExprData::Index(i) => {
            println!();
            print_expr(&i.obj, indent + 2);
            print_expr(&i.index, indent + 2);
        }
        ExprData::Slice(s) => {
            println!(" inclusive={}", s.inclusive);
            print_expr(&s.obj, indent + 2);
            if let Some(e) = &s.start { print_expr(e, indent + 2); }
            if let Some(e) = &s.end { print_expr(e, indent + 2); }
        }
        ExprData::VarDecl(v) => {
            println!(" name={} mut={}", v.name, v.is_mut);
            if let Some(t) = &v.type_name { println!("{}  type={}", pad, t); }
            if let Some(i) = &v.init { print_expr(i, indent + 2); }
        }
        ExprData::Assign(a) => {
            println!();
            print_expr(&a.target, indent + 2);
            print_expr(&a.value, indent + 2);
        }
        ExprData::ExprStmt(e) => {
            println!();
            print_expr(&e.expr, indent + 2);
        }
        ExprData::Block(b) => {
            println!(" stmts={}", b.stmts.len());
            for s in &b.stmts { print_expr(s, indent + 2); }
        }
        ExprData::IfStmt(i) => {
            println!();
            print_expr(&i.cond, indent + 2);
            print_expr(&i.then_block, indent + 2);
            if let Some(e) = &i.else_block { print_expr(e, indent + 2); }
        }
        ExprData::WhileStmt(w) => {
            println!();
            print_expr(&w.cond, indent + 2);
            print_expr(&w.body, indent + 2);
        }
        ExprData::ForStmt(f) => {
            println!(" var={} kind={:?}", f.var_name, f.kind);
            if let Some(e) = &f.iterable { print_expr(e, indent + 2); }
            if let Some(e) = &f.start { print_expr(e, indent + 2); }
            if let Some(e) = &f.end { print_expr(e, indent + 2); }
            if let Some(e) = &f.step { print_expr(e, indent + 2); }
            print_expr(&f.body, indent + 2);
        }
        ExprData::ReturnStmt(r) => {
            println!();
            if let Some(e) = &r.value { print_expr(e, indent + 2); }
        }
        ExprData::BreakStmt => println!("  <break>"),
        ExprData::ContinueStmt => println!("  <continue>"),
        ExprData::MatchStmt(m) => {
            println!(" arms={}", m.arms.len());
            for arm in &m.arms {
                println!("{}  pattern={:?}", pad, arm.pattern);
                print_expr(&arm.body, indent + 2);
            }
        }
        ExprData::DeferStmt(d) => {
            println!();
            print_expr(&d.expr, indent + 2);
        }
        ExprData::Function(f) => {
            println!(" name={} params={}", f.name, f.params.len());
            for p in &f.params { println!("{}  param={}", pad, p.name); }
            print_expr(&f.body, indent + 2);
        }
        ExprData::StructDecl(s) => {
            println!(" name={} fields={}", s.name, s.fields.len());
            for fd in &s.fields { println!("{}  field={}:{}", pad, fd.name, fd.type_name.as_deref().unwrap_or("?")); }
        }
        ExprData::EnumDecl(e) => {
            println!(" name={} variants={}", e.name, e.variants.len());
            for v in &e.variants { println!("{}  variant={}", pad, v.name); }
        }
        ExprData::Program(p) => {
            println!(" items={}", p.items.len());
            for it in &p.items { print_expr(it, indent + 2); }
        }
        ExprData::Error => println!("{}  <error>", pad),
    }
}

// ============ 测试 ============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_lit() {
        let e = new_int_lit(42, 1, 1);
        assert_eq!(e.kind, Kind::IntLit);
        if let ExprData::IntLit(v) = e.data { assert_eq!(v, 42); } else { panic!(); }
    }

    #[test]
    fn test_float_lit() {
        // 用 2.5 而非 3.14 避免触发 clippy::approx_constant
        let e = new_float_lit(2.5, 1, 1);
        assert_eq!(e.kind, Kind::FloatLit);
        if let ExprData::FloatLit(v) = e.data { assert!((v - 2.5).abs() < 1e-9); } else { panic!(); }
    }

    #[test]
    fn test_string_lit() {
        let e = new_string_lit(string_lit_simple("hi"), 1, 1);
        assert_eq!(e.kind, Kind::StringLit);
    }

    #[test]
    fn test_binary() {
        let e = new_binary(BinOp::Add, new_int_lit(1, 1, 1), new_int_lit(2, 1, 1), 1, 1);
        assert_eq!(e.kind, Kind::BinaryOp);
    }

    #[test]
    fn test_var_decl() {
        let e = new_var_decl("x".into(), Some("int".into()), false, Some(new_int_lit(0, 1, 1)), 1, 1);
        assert_eq!(e.kind, Kind::VarDecl);
    }

    #[test]
    fn test_block() {
        let e = new_block(vec![], 1, 1);
        assert_eq!(e.kind, Kind::Block);
    }

    #[test]
    fn test_program() {
        let e = new_program(vec![], 1, 1);
        assert_eq!(e.kind, Kind::Program);
    }

    #[test]
    fn test_complex_tree() {
        // let x = 1 + 2
        let inner = new_binary(BinOp::Add, new_int_lit(1, 1, 5), new_int_lit(2, 1, 9), 1, 7);
        let vd = new_var_decl("x".into(), None, false, Some(inner), 1, 5);
        let prog = new_program(vec![vd], 1, 1);
        assert_eq!(prog.kind, Kind::Program);
    }
}