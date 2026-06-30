//! Tree-walking 解释器
//!
//! v0.1 实现：表达式求值、控制流、函数调用、match、内置 print。
//! 错误处理使用结构化 `EvalError` 枚举（替代 v0.0 的字符串错误），
//! 既支持程序化 `match` 又支持 `Display` 友好输出。

use crate::ast::*;
use crate::env::Env;
use crate::value::{Value, ValueKind, val_int, val_float, val_bool, val_string, val_nil};

/// 求值状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvalStatus {
    Ok,
    Error,
    Break,
    Return,
}

/// 求值错误（结构化）
///
/// 每个变体代表一类语义错误。消费者可以 `match EvalError::Xxx` 做程序化处理，
/// 也可以通过 `Display` 拿到中文友好信息。
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    /// 当前解释器不支持的语言特性（field/index/slice 等）
    UnsupportedFeature(&'static str),
    /// VarDecl 出现在表达式位置
    VarDeclAsExpression,
    /// 求值器遇到未实现的表达式类型
    UnsupportedExpression,
    /// 除法或取模时除数为零
    DivisionByZero,
    /// Concat 运算符仅支持字符串
    ConcatNonString,
    /// 调用了未定义的函数
    UnknownFunction(String),
    /// 尝试调用非函数值
    NotAFunction,
    /// 函数实参与形参数量不匹配
    ArgumentCountMismatch { expected: usize, got: usize },
    /// break 出现在循环外
    BreakOutsideLoop,
    /// 给未定义的变量赋值
    UnknownVariable(String),
    /// 赋值目标不是合法的变量
    AssignTargetInvalid,
    /// for-in 循环缺少 iterable 表达式
    ForInMissingIterable,
    /// for-range 循环缺少 start 表达式
    ForRangeMissingStart,
    /// for-range 循环缺少 end 表达式
    ForRangeMissingEnd,
    /// 在某类型上调用了不存在的方法
    UnknownMethod { receiver_type: String, method: String },
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedFeature(s) => write!(f, "{} 暂未在 v0.1 解释器中实现", s),
            Self::VarDeclAsExpression => write!(f, "VarDecl 不能作为表达式使用"),
            Self::UnsupportedExpression => write!(f, "不支持的表达式"),
            Self::DivisionByZero => write!(f, "除数为零"),
            Self::ConcatNonString => write!(f, "concat 运算符仅支持字符串"),
            Self::UnknownFunction(name) => write!(f, "未找到函数 '{}'", name),
            Self::NotAFunction => write!(f, "尝试调用非函数值"),
            Self::ArgumentCountMismatch { expected, got } => {
                write!(f, "参数数量不匹配：期望 {} 个，实参 {} 个", expected, got)
            }
            Self::BreakOutsideLoop => write!(f, "break 出现在循环外"),
            Self::UnknownVariable(name) => write!(f, "未定义变量 '{}'", name),
            Self::AssignTargetInvalid => write!(f, "赋值目标必须是变量"),
            Self::ForInMissingIterable => write!(f, "for-in 缺少 iterable 表达式"),
            Self::ForRangeMissingStart => write!(f, "for-range 缺少 start 表达式"),
            Self::ForRangeMissingEnd => write!(f, "for-range 缺少 end 表达式"),
            Self::UnknownMethod { receiver_type, method } => {
                write!(f, "{} 类型没有方法 '{}'", receiver_type, method)
            }
        }
    }
}

/// 求值结果
#[derive(Debug, Clone)]
pub struct EvalResult {
    pub status: EvalStatus,
    pub value: Value,
    pub error: Option<EvalError>,
}

pub fn eval_ok(v: Value) -> EvalResult {
    EvalResult { status: EvalStatus::Ok, value: v, error: None }
}

pub fn eval_err(e: EvalError) -> EvalResult {
    EvalResult { status: EvalStatus::Error, value: val_nil(), error: Some(e) }
}

/// 函数注册表
#[derive(Debug, Clone)]
pub struct FuncReg {
    funcs: Vec<Expr>,
}

impl FuncReg {
    pub fn new() -> Self {
        FuncReg { funcs: Vec::new() }
    }
    pub fn add(&mut self, func: Expr) {
        self.funcs.push(func);
    }
    pub fn lookup(&self, name: &str) -> Option<&Expr> {
        for f in &self.funcs {
            if let ExprData::Function(fd) = &f.data {
                if fd.name == name {
                    return Some(f);
                }
            }
        }
        None
    }
}

impl Default for FuncReg {
    fn default() -> Self { Self::new() }
}

/// 解释器
pub struct Interpreter {
    env: Env,
    regs: FuncReg,
    current_regs: Option<*const FuncReg>,
}

// SAFETY: `Interpreter` 当前仅在主线程的 `exec_program` 同步调用中使用（无并发）。
// `Env` 和 `FuncReg` 都使用 `HashMap`（非线程安全），但只要保持单线程访问就 Send 安全。
// 未来若需要并发执行，必须重审：Env 需要换 `RwLock<HashMap>`，并显式实现 Sync。
unsafe impl Send for Interpreter {}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            env: Env::new(),
            regs: FuncReg::new(),
            current_regs: None,
        }
    }

    /// 求值表达式
    pub fn eval_expr(&mut self, expr: &Expr) -> EvalResult {
        self.eval_expr_inner(expr)
    }

    fn eval_expr_inner(&mut self, expr: &Expr) -> EvalResult {
        match &expr.data {
            ExprData::IntLit(v) => eval_ok(val_int(*v)),
            ExprData::FloatLit(v) => eval_ok(val_float(*v)),
            ExprData::BoolLit(v) => eval_ok(val_bool(*v)),
            ExprData::CharLit(v) => eval_ok(val_int(*v as i64)),
            ExprData::StringLit(s) => {
                // v0.5 简化：字符串字面量在词法层已是 InterpPart 列表
                // 运行时拼接：text 段直接用，expr 段求值后转字符串
                let mut result = String::new();
                for p in &s.parts {
                    match p {
                        crate::ast::InterpPart::Text(t) => result.push_str(t),
                        crate::ast::InterpPart::Expr(e) => {
                            let r = self.eval_expr_inner(e);
                            if r.status != EvalStatus::Ok {
                                return r;
                            }
                            result.push_str(&r.value.to_string());
                        }
                    }
                }
                eval_ok(val_string(result))
            }
            ExprData::Ident(name) => eval_ok(self.env.get(name)),
            ExprData::BinaryOp(b) => self.eval_binary(b),
            ExprData::UnaryOp(u) => self.eval_unary(u),
            ExprData::Call(c) => self.eval_call(c),
            ExprData::FieldAccess(_f) => eval_err(EvalError::UnsupportedFeature("field access")),
            ExprData::Index(_i) => eval_err(EvalError::UnsupportedFeature("index")),
            ExprData::Slice(_s) => eval_err(EvalError::UnsupportedFeature("slice")),
            ExprData::VarDecl(_v) => {
                // v0.5：VarDecl 在表达式位置不应出现（由 exec_stmt 处理）；
                // 这里仅作为兜底，保留占位以避免崩溃。
                eval_err(EvalError::VarDeclAsExpression)
            }
            _ => eval_err(EvalError::UnsupportedExpression),
        }
    }

    fn eval_binary(&mut self, b: &BinaryOp) -> EvalResult {
        let lhs = self.eval_expr_inner(&b.lhs);
        if lhs.status != EvalStatus::Ok { return lhs; }
        let rhs = self.eval_expr_inner(&b.rhs);
        if rhs.status != EvalStatus::Ok {
            return EvalResult { status: rhs.status, ..rhs };
        }

        match b.op {
            BinOp::Add => {
                if matches!(lhs.value, Value::String(_)) && matches!(rhs.value, Value::String(_)) {
                    if let (Value::String(a), Value::String(c)) = (&lhs.value, &rhs.value) {
                        return eval_ok(val_string(format!("{}{}", a, c)));
                    }
                }
                if matches!(lhs.value.kind(), ValueKind::Float) || matches!(rhs.value.kind(), ValueKind::Float) {
                    let r = lhs.value.to_float() + rhs.value.to_float();
                    return eval_ok(val_float(r));
                }
                eval_ok(val_int(int_arith(BinOp::Add, lhs.value.to_int(), rhs.value.to_int())))
            }
            BinOp::Sub => {
                if matches!(lhs.value.kind(), ValueKind::Float) || matches!(rhs.value.kind(), ValueKind::Float) {
                    return eval_ok(val_float(lhs.value.to_float() - rhs.value.to_float()));
                }
                eval_ok(val_int(int_arith(BinOp::Sub, lhs.value.to_int(), rhs.value.to_int())))
            }
            BinOp::Mul => {
                if matches!(lhs.value.kind(), ValueKind::Float) || matches!(rhs.value.kind(), ValueKind::Float) {
                    return eval_ok(val_float(lhs.value.to_float() * rhs.value.to_float()));
                }
                eval_ok(val_int(int_arith(BinOp::Mul, lhs.value.to_int(), rhs.value.to_int())))
            }
            BinOp::Div => {
                let b = rhs.value.to_float();
                if b == 0.0 {
                    return eval_err(EvalError::DivisionByZero);
                }
                if matches!(lhs.value.kind(), ValueKind::Float) || matches!(rhs.value.kind(), ValueKind::Float) {
                    return eval_ok(val_float(lhs.value.to_float() / b));
                }
                eval_ok(val_int(lhs.value.to_int() / rhs.value.to_int()))
            }
            BinOp::Mod => {
                let b = rhs.value.to_int();
                if b == 0 {
                    return eval_err(EvalError::DivisionByZero);
                }
                eval_ok(val_int(lhs.value.to_int() % b))
            }
            BinOp::Eq => eval_ok(val_bool(lhs.value.equal(&rhs.value))),
            BinOp::Neq => eval_ok(val_bool(!lhs.value.equal(&rhs.value))),
            BinOp::Lt => eval_ok(val_bool(lhs.value.to_float() < rhs.value.to_float())),
            BinOp::Le => eval_ok(val_bool(lhs.value.to_float() <= rhs.value.to_float())),
            BinOp::Gt => eval_ok(val_bool(lhs.value.to_float() > rhs.value.to_float())),
            BinOp::Ge => eval_ok(val_bool(lhs.value.to_float() >= rhs.value.to_float())),
            BinOp::And => {
                // 短路：左 false 时不评估右
                let lb = lhs.value.to_bool();
                if !lb {
                    return eval_ok(val_bool(false));
                }
                eval_ok(val_bool(rhs.value.to_bool()))
            }
            BinOp::Or => {
                // 短路：左 true 时不评估右
                let lb = lhs.value.to_bool();
                if lb {
                    return eval_ok(val_bool(true));
                }
                eval_ok(val_bool(rhs.value.to_bool()))
            }
            BinOp::BitAnd => eval_ok(val_int(lhs.value.to_int() & rhs.value.to_int())),
            BinOp::BitOr => eval_ok(val_int(lhs.value.to_int() | rhs.value.to_int())),
            BinOp::BitXor => eval_ok(val_int(lhs.value.to_int() ^ rhs.value.to_int())),
            BinOp::Shl => eval_ok(val_int(lhs.value.to_int() << rhs.value.to_int())),
            BinOp::Shr => eval_ok(val_int(lhs.value.to_int() >> rhs.value.to_int())),
            BinOp::Concat => {
                if let (Value::String(a), Value::String(c)) = (&lhs.value, &rhs.value) {
                    return eval_ok(val_string(format!("{}{}", a, c)));
                }
                eval_err(EvalError::ConcatNonString)
            }
        }
    }

    fn eval_unary(&mut self, u: &UnaryOp) -> EvalResult {
        let operand = self.eval_expr_inner(&u.operand);
        if operand.status != EvalStatus::Ok { return operand; }
        match u.op {
            UnOp::Neg => {
                if matches!(operand.value.kind(), ValueKind::Float) {
                    eval_ok(val_float(-operand.value.to_float()))
                } else {
                    eval_ok(val_int(int_neg(operand.value.to_int())))
                }
            }
            UnOp::Not => eval_ok(val_bool(!operand.value.to_bool())),
            UnOp::BitNot => eval_ok(val_int(!operand.value.to_int())),
        }
    }

    fn eval_call(&mut self, c: &Call) -> EvalResult {
        // 内置 print
        if let ExprData::Ident(name) = &c.func.data {
            if name == "print" {
                return self.eval_print(c);
            }
        }
        // 方法调用：c.func 是 FieldAccess（v0.11+，A4 字符串基础操作）
        if let ExprData::FieldAccess(fa) = &c.func.data {
            let recv = self.eval_expr_inner(&fa.obj);
            if recv.status != EvalStatus::Ok { return recv; }
            if let Some(r) = self.eval_method_call(&recv.value, &fa.field, &c.args) {
                return r;
            }
            return eval_err(EvalError::UnknownMethod {
                receiver_type: recv.value.kind().as_str().to_string(),
                method: fa.field.clone(),
            });
        }
        // 用户函数
        let func_expr = {
            if let ExprData::Ident(name) = &c.func.data {
                self.regs.lookup(name).cloned()
            } else {
                None
            }
        };
        let func = match func_expr {
            Some(f) => f,
            None => {
                let name = if let ExprData::Ident(n) = &c.func.data {
                    n.clone()
                } else {
                    String::from("<anonymous>")
                };
                return eval_err(EvalError::UnknownFunction(name));
            }
        };
        let func_data = match &func.data {
            ExprData::Function(fd) => fd.clone(),
            _ => return eval_err(EvalError::NotAFunction),
        };
        if c.args.len() != func_data.params.len() {
            return eval_err(EvalError::ArgumentCountMismatch {
                expected: func_data.params.len(),
                got: c.args.len(),
            });
        }
        // 求值参数
        let mut arg_vals = Vec::new();
        for arg in &c.args {
            let r = self.eval_expr_inner(arg);
            if r.status != EvalStatus::Ok { return r; }
            arg_vals.push(r.value);
        }
        // 创建子作用域
        let mut scope = Env::child(self.env.clone());
        for (p, v) in func_data.params.iter().zip(arg_vals.iter()) {
            scope.define(&p.name, v.clone());
        }
        // 执行函数体
        let saved_env = std::mem::replace(&mut self.env, scope);
        let body_result = self.exec_stmt(&func_data.body);
        self.env = saved_env;
        match body_result.status {
            EvalStatus::Return => EvalResult { status: EvalStatus::Ok, ..body_result },
            EvalStatus::Break => eval_err(EvalError::BreakOutsideLoop),
            EvalStatus::Error => body_result,
            EvalStatus::Ok => body_result,
        }
    }

    fn eval_print(&mut self, c: &Call) -> EvalResult {
        for (i, arg) in c.args.iter().enumerate() {
            let r = self.eval_expr_inner(arg);
            if r.status != EvalStatus::Ok { return r; }
            if i > 0 { print!(" "); }
            print!("{}", r.value);
        }
        println!();
        eval_ok(val_nil())
    }

    /// 执行单条语句
    pub fn exec_stmt(&mut self, stmt: &Expr) -> EvalResult {
        match &stmt.data {
            ExprData::VarDecl(v) => {
                let init = if let Some(i) = &v.init {
                    let r = self.eval_expr_inner(i);
                    if r.status != EvalStatus::Ok { return r; }
                    Some(r.value)
                } else {
                    None
                };
                let val = init.unwrap_or_else(val_nil);
                self.env.define(&v.name, val);
                eval_ok(val_nil())
            }
            ExprData::Assign(a) => {
                let val = self.eval_expr_inner(&a.value);
                if val.status != EvalStatus::Ok { return val; }
                if let ExprData::Ident(name) = &a.target.data {
                    if !self.env.set(name, val.value.clone()) {
                        return eval_err(EvalError::UnknownVariable(name.clone()));
                    }
                    eval_ok(val.value)
                } else {
                    eval_err(EvalError::AssignTargetInvalid)
                }
            }
            ExprData::ExprStmt(e) => {
                // 特殊处理 ExprStmt 包装的 Assign：assign 是语句而非表达式
                if matches!(&e.expr.data, ExprData::Assign(_)) {
                    self.exec_stmt(&e.expr)
                } else {
                    self.eval_expr_inner(&e.expr)
                }
            }
            ExprData::Block(b) => self.exec_block(&b.stmts),
            ExprData::IfStmt(i) => {
                let cond = self.eval_expr_inner(&i.cond);
                if cond.status != EvalStatus::Ok { return cond; }
                if cond.value.to_bool() {
                    self.exec_stmt(&i.then_block)
                } else if let Some(e) = &i.else_block {
                    self.exec_stmt(e)
                } else {
                    eval_ok(val_nil())
                }
            }
            ExprData::WhileStmt(w) => loop {
                let cond = self.eval_expr_inner(&w.cond);
                if cond.status != EvalStatus::Ok { return cond; }
                if !cond.value.to_bool() { break eval_ok(val_nil()); }
                let r = self.exec_stmt(&w.body);
                if r.status == EvalStatus::Break { break eval_ok(val_nil()); }
                if r.status != EvalStatus::Ok { return r; }
            },
            ExprData::ForStmt(f) => self.exec_for(f),
            ExprData::ReturnStmt(r) => {
                let v = if let Some(v) = &r.value {
                    let r = self.eval_expr_inner(v);
                    if r.status != EvalStatus::Ok { return r; }
                    Some(r.value)
                } else { None };
                EvalResult {
                    status: EvalStatus::Return,
                    value: v.unwrap_or_else(val_nil),
                    error: None,
                }
            }
            ExprData::BreakStmt => EvalResult {
                status: EvalStatus::Break,
                value: val_nil(),
                error: None,
            },
            ExprData::ContinueStmt => eval_ok(val_nil()),
            ExprData::MatchStmt(m) => self.exec_match(m),
            _ => self.eval_expr_inner(stmt),
        }
    }

    fn exec_block(&mut self, stmts: &[Expr]) -> EvalResult {
        let mut last = eval_ok(val_nil());
        for s in stmts {
            last = self.exec_stmt(s);
            if last.status != EvalStatus::Ok {
                return last;
            }
        }
        last
    }

    fn exec_for(&mut self, f: &ForStmt) -> EvalResult {
        match &f.kind {
            ForKind::ForIn => {
                let iterable = match &f.iterable {
                    Some(e) => e,
                    None => return eval_err(EvalError::ForInMissingIterable),
                };
                let iter = self.eval_expr_inner(iterable);
                if iter.status != EvalStatus::Ok { return iter; }
                let parent_env = self.env.clone();
                let saved_env = std::mem::replace(&mut self.env, Env::child(parent_env));
                match &iter.value {
                    Value::String(s) => {
                        for ch in s.chars() {
                            self.env.define(&f.var_name, val_int(ch as i64));
                            let r = self.exec_stmt(&f.body);
                            if r.status == EvalStatus::Break { break; }
                            if r.status != EvalStatus::Ok { self.env = saved_env; return r; }
                        }
                    }
                    Value::Int(n) => {
                        self.env.define(&f.var_name, val_int(*n));
                        let r = self.exec_stmt(&f.body);
                        if r.status != EvalStatus::Ok { self.env = saved_env; return r; }
                    }
                    _ => {
                        self.env.define(&f.var_name, val_int(0));
                        let r = self.exec_stmt(&f.body);
                        if r.status != EvalStatus::Ok { self.env = saved_env; return r; }
                    }
                }
                self.env = saved_env;
                eval_ok(val_nil())
            }
            ForKind::ForRange => {
                let start_expr = match &f.start {
                    Some(e) => e,
                    None => return eval_err(EvalError::ForRangeMissingStart),
                };
                let end_expr = match &f.end {
                    Some(e) => e,
                    None => return eval_err(EvalError::ForRangeMissingEnd),
                };
                let start = self.eval_expr_inner(start_expr);
                if start.status != EvalStatus::Ok { return start; }
                let end = self.eval_expr_inner(end_expr);
                if end.status != EvalStatus::Ok { return end; }
                let step_val = if let Some(s) = &f.step {
                    let r = self.eval_expr_inner(s);
                    if r.status != EvalStatus::Ok { return r; }
                    r.value.to_int()
                } else {
                    1
                };
                let s = start.value.to_int();
                let e = end.value.to_int();
                let step = if step_val == 0 { 1 } else { step_val };
                let parent_env = self.env.clone();
                let saved_env = std::mem::replace(&mut self.env, Env::child(parent_env));
                let mut i = s;
                loop {
                    let done = if step >= 0 { i > e } else { i < e };
                    if done { break; }
                    self.env.define(&f.var_name, val_int(i));
                    let r = self.exec_stmt(&f.body);
                    if r.status == EvalStatus::Break { break; }
                    if r.status != EvalStatus::Ok { self.env = saved_env; return r; }
                    i += step;
                }
                self.env = saved_env;
                eval_ok(val_nil())
            }
        }
    }

    fn exec_match(&mut self, m: &MatchStmt) -> EvalResult {
        let val = self.eval_expr_inner(&m.scrutinee);
        if val.status != EvalStatus::Ok { return val; }
        for arm in &m.arms {
            if match_pattern(&arm.pattern, &val.value) {
                return self.exec_stmt(&arm.body);
            }
        }
        eval_ok(val_nil())
    }

    /// 执行程序
    ///
    /// 返回完整的 `EvalResult`，调用方可通过 `result.error` 拿到结构化错误。
    pub fn exec_program(&mut self, prog: &Expr) -> EvalResult {
        if let ExprData::Program(p) = &prog.data {
            // 注册所有函数
            for item in &p.items {
                if matches!(item.data, ExprData::Function(_)) {
                    self.regs.add(item.clone());
                }
            }
            // 找 main
            let main = self.regs.lookup("main").cloned();
            match main {
                Some(m) => {
                    if let ExprData::Function(fd) = &m.data {
                        return self.exec_stmt(&fd.body);
                    }
                    eval_err(EvalError::UnsupportedExpression) // 不是 Program/Function
                }
                None => eval_err(EvalError::UnknownFunction("main".into())),
            }
        } else {
            eval_err(EvalError::UnsupportedExpression)
        }
    }
}

/// 模式匹配
fn match_pattern(pat: &str, v: &Value) -> bool {
    if pat == "_" { return true; }
    match v.kind() {
        ValueKind::Int => v.to_int() == pat.parse::<i64>().unwrap_or(i64::MIN),
        ValueKind::Float => (v.to_float() - pat.parse::<f64>().unwrap_or(0.0)).abs() < 1e-9,
        ValueKind::Bool => {
            if pat == "true" { v.to_bool() } else if pat == "false" { !v.to_bool() } else { false }
        }
        ValueKind::String => v.to_string() == pat,
        ValueKind::Nil => pat == "nil",
    }
}

// ============ A3 整数溢出检查 ============
//
// 行为矩阵（v0.11+）：
// - debug 模式（`cfg!(debug_assertions)` 为 true）：i64 加减乘/取负溢出时 panic，错误信息 "integer overflow in <op>"
// - release 模式：保持 v0.10 的 wrap 行为（向后兼容，零开销）
// - 浮点不检查（Inf/NaN 是 IEEE-754 合理行为）
// - 除零保持现有 EvalError::DivisionByZero（不 panic）

/// 不分模式的整数算术：返回 `Some(v)` 表示正常，`None` 表示溢出。
///
/// 该函数是模式无关的，便于在测试中跨模式验证溢出判定。
fn checked_int_arith(op: BinOp, a: i64, b: i64) -> Option<i64> {
    match op {
        BinOp::Add => a.checked_add(b),
        BinOp::Sub => a.checked_sub(b),
        BinOp::Mul => a.checked_mul(b),
        _ => None,
    }
}

/// 整数算术（模式相关）：debug 溢出 panic，release wrap。
///
/// `cfg!(debug_assertions)` 是编译期常量（由 `--release`/profile 决定），
/// 编译后只剩一条分支，因此没有运行时分支开销。
fn int_arith(op: BinOp, a: i64, b: i64) -> i64 {
    if cfg!(debug_assertions) {
        match checked_int_arith(op, a, b) {
            Some(v) => v,
            None => panic!("integer overflow in {}", op),
        }
    } else {
        match op {
            BinOp::Add => a.wrapping_add(b),
            BinOp::Sub => a.wrapping_sub(b),
            BinOp::Mul => a.wrapping_mul(b),
            _ => 0,
        }
    }
}

/// 整数取负（模式相关）：debug 溢出 panic（i64::MIN），release wrap。
fn int_neg(a: i64) -> i64 {
    if cfg!(debug_assertions) {
        match a.checked_neg() {
            Some(v) => v,
            None => panic!("integer overflow in unary -"),
        }
    } else {
        a.wrapping_neg()
    }
}

// ============ A4 字符串基础操作 ============
//
// 设计：方法调用通过 `Call { func: FieldAccess { obj, field }, args }` 模式分发。
// 当前仅实现 `Value::String` 的方法（v0.11 第一批）：
//   len / len_bytes / concat / contains / starts_with / ends_with / slice
// 其他类型的方法在后续版本按 trait / extension 引入。

impl Interpreter {
    /// 方法分发入口。
    ///
    /// 返回 `Some(result)` 表示方法被识别（无论结果成功还是错误），
    /// 返回 `None` 表示该方法在接收者类型上不存在，外层应报 `UnknownMethod`。
    fn eval_method_call(
        &mut self,
        receiver: &Value,
        method: &str,
        args: &[Expr],
    ) -> Option<EvalResult> {
        match receiver {
            Value::String(s) => self.eval_string_method(s, method, args),
            _ => None,
        }
    }

    /// 字符串方法分派。返回 `None` 表示方法名不在本类型的方法表里。
    fn eval_string_method(
        &mut self,
        s: &str,
        method: &str,
        args: &[Expr],
    ) -> Option<EvalResult> {
        // 先评估参数（与 Python 风格一致：先 receiver，再 args）
        let mut arg_vals: Vec<Value> = Vec::with_capacity(args.len());
        for arg in args {
            let r = self.eval_expr_inner(arg);
            if r.status != EvalStatus::Ok { return Some(r); }
            arg_vals.push(r.value);
        }

        let arity_err = |expected: usize| -> EvalResult {
            eval_err(EvalError::ArgumentCountMismatch {
                expected,
                got: arg_vals.len(),
            })
        };

        match method {
            "len" | "length" => {
                if !arg_vals.is_empty() { return Some(arity_err(0)); }
                Some(eval_ok(val_int(s.chars().count() as i64)))
            }
            "len_bytes" => {
                if !arg_vals.is_empty() { return Some(arity_err(0)); }
                Some(eval_ok(val_int(s.len() as i64)))
            }
            "concat" => {
                if arg_vals.len() != 1 { return Some(arity_err(1)); }
                let other = arg_vals[0].to_string();
                Some(eval_ok(val_string(format!("{}{}", s, other))))
            }
            "contains" => {
                if arg_vals.len() != 1 { return Some(arity_err(1)); }
                let needle = arg_vals[0].to_string();
                Some(eval_ok(val_bool(s.contains(&needle))))
            }
            "starts_with" => {
                if arg_vals.len() != 1 { return Some(arity_err(1)); }
                let prefix = arg_vals[0].to_string();
                Some(eval_ok(val_bool(s.starts_with(&prefix))))
            }
            "ends_with" => {
                if arg_vals.len() != 1 { return Some(arity_err(1)); }
                let suffix = arg_vals[0].to_string();
                Some(eval_ok(val_bool(s.ends_with(&suffix))))
            }
            "slice" => {
                if arg_vals.len() != 2 { return Some(arity_err(2)); }
                let start = arg_vals[0].to_int();
                let end = arg_vals[1].to_int();
                // UTF-8 安全切片：先收 chars 再切
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len() as i64;
                let s_idx = start.max(0).min(len);
                let e_idx = end.max(0).min(len);
                let sliced: String = if s_idx <= e_idx {
                    chars[s_idx as usize..e_idx as usize].iter().collect()
                } else {
                    String::new()
                };
                Some(eval_ok(val_string(sliced)))
            }
            _ => None, // 未知方法：让外层报 UnknownMethod
        }
    }
}

#[cfg(test)]
mod tests {
    //! 单元测试覆盖 A3（整数溢出检查）和 A4（字符串基础操作）。

    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    /// 解析 + 求值入口（用于测试）
    fn run(src: &str) -> EvalResult {
        let lex = Lexer::new(src);
        let mut p = Parser::new(lex);
        let prog = p.parse_program();
        let mut interp = Interpreter::new();
        interp.exec_program(&prog)
    }

    // ============ A3: 整数溢出 ============

    /// 正常加法不溢出 → 两个模式都应正常工作（解释器端到端验证）
    #[test]
    fn test_int_add_normal() {
        let r = run("func main() { 1 + 2 }");
        expect_int(&r, 3);
    }

    /// `checked_int_arith` 是模式无关的：可直接断言溢出行为
    #[test]
    fn test_checked_int_arith_overflow() {
        assert_eq!(checked_int_arith(BinOp::Add, i64::MAX, 1), None);
        assert_eq!(checked_int_arith(BinOp::Sub, i64::MIN, 1), None);
        assert_eq!(checked_int_arith(BinOp::Mul, i64::MAX, 2), None);
        assert_eq!(checked_int_arith(BinOp::Add, 5, 3), Some(8));
        assert_eq!(checked_int_arith(BinOp::Mul, -3, 7), Some(-21));
        assert_eq!(checked_int_arith(BinOp::Sub, 0, 5), Some(-5));
    }

    /// debug 模式：i64::MAX + 1 必须 panic，消息含 "integer overflow"
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "integer overflow")]
    fn test_int_overflow_add_panics_in_debug() {
        // 9223372036854775807 == i64::MAX（在词法层能正确解析为 i64::MAX）
        let _ = run("func main() { let x = 9223372036854775807; x + 1 }");
    }

    /// debug 模式：i64::MAX * 2 也必须 panic
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "integer overflow")]
    fn test_int_overflow_mul_panics_in_debug() {
        let _ = run("func main() { let x = 9223372036854775807; x * 2 }");
    }

    /// debug 模式：i64::MIN - 1 panic
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "integer overflow")]
    fn test_int_overflow_sub_panics_in_debug() {
        // i64::MIN = -9223372036854775808。源码里写 -9223372036854775808
        // 会被词法切成 -IntLit(9223372036854775808)，而 9223372036854775808 溢出
        // parse::<i64>，默认 0。绕路：用 let x = -9223372036854775807 - 1。
        let _ = run("func main() { let x = -9223372036854775807 - 1; x - 1 }");
    }

    /// release 模式：i64::MAX + 1 静默 wrap 到 i64::MIN
    #[cfg(not(debug_assertions))]
    #[test]
    fn test_int_overflow_add_wraps_in_release() {
        let r = run("func main() { let x = 9223372036854775807; x + 1 }");
        expect_int(&r, i64::MIN);
        let r = run("func main() { let x = 9223372036854775807; x * 2 }");
        expect_int(&r, -2);
    }

    /// debug 模式：直接调用 int_neg(i64::MIN) panic（绕开词法层无法表达 i64::MIN 字面量的问题）
    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "integer overflow")]
    fn test_int_neg_min_panics_in_debug() {
        let _ = int_neg(i64::MIN);
    }

    /// 浮点加法不检查溢出（Inf 是 IEEE-754 合理行为）
    #[test]
    fn test_float_overflow_no_check() {
        let r = run("func main() { let x = 1.7e308; x + x }");
        assert_eq!(r.status, EvalStatus::Ok);
        // Inf 是预期结果
    }

    /// 除零已有错误处理，不能因为 A3 改变
    #[test]
    fn test_div_by_zero_still_errors() {
        let r = run("func main() { 10 / 0 }");
        assert_eq!(r.status, EvalStatus::Error);
        assert_eq!(r.error, Some(EvalError::DivisionByZero));
    }

    // ============ A4: 字符串基础操作 ============

    fn expect_int(r: &EvalResult, expected: i64) {
        assert_eq!(r.status, EvalStatus::Ok, "eval failed: {:?}", r.error);
        match &r.value {
            Value::Int(n) => assert_eq!(*n, expected, "value mismatch: got {}", n),
            other => panic!("expected int, got {:?}", other),
        }
    }

    fn expect_string(r: &EvalResult, expected: &str) {
        assert_eq!(r.status, EvalStatus::Ok, "eval failed: {:?}", r.error);
        match &r.value {
            Value::String(s) => assert_eq!(s, expected),
            other => panic!("expected string, got {:?}", other),
        }
    }

    fn expect_bool(r: &EvalResult, expected: bool) {
        assert_eq!(r.status, EvalStatus::Ok);
        match &r.value {
            Value::Bool(b) => assert_eq!(*b, expected),
            other => panic!("expected bool, got {:?}", other),
        }
    }

    #[test]
    fn test_str_len_chars() {
        let r = run(r#"func main() { let s = "你好"; s.len() }"#);
        expect_int(&r, 2);
    }

    #[test]
    fn test_str_len_bytes() {
        let r = run(r#"func main() { let s = "你好"; s.len_bytes() }"#);
        expect_int(&r, 6);
    }

    #[test]
    fn test_str_concat() {
        let r = run(r#"func main() { let s = "你好"; s.concat("世界") }"#);
        expect_string(&r, "你好世界");
    }

    #[test]
    fn test_str_contains() {
        let r = run(r#"func main() { let s = "你好世界"; s.contains("好世") }"#);
        expect_bool(&r, true);
        let r = run(r#"func main() { let s = "你好世界"; s.contains("再见") }"#);
        expect_bool(&r, false);
    }

    #[test]
    fn test_str_starts_ends_with() {
        let r = run(r#"func main() { let s = "你好世界"; s.starts_with("你好") }"#);
        expect_bool(&r, true);
        let r = run(r#"func main() { let s = "你好世界"; s.ends_with("世界") }"#);
        expect_bool(&r, true);
        let r = run(r#"func main() { let s = "你好世界"; s.starts_with("世界") }"#);
        expect_bool(&r, false);
    }

    #[test]
    fn test_str_slice_half_open() {
        // 切片是半开 [start, end)
        let r = run(r#"func main() { let s = "你好世界"; s.slice(0, 2) }"#);
        expect_string(&r, "你好");
        let r = run(r#"func main() { let s = "你好世界"; s.slice(2, 4) }"#);
        expect_string(&r, "世界");
        // 空切片
        let r = run(r#"func main() { let s = "你好"; s.slice(1, 1) }"#);
        expect_string(&r, "");
        // 越界 end 自动 clamp
        let r = run(r#"func main() { let s = "你好"; s.slice(0, 999) }"#);
        expect_string(&r, "你好");
    }

    #[test]
    fn test_str_unknown_method_errors() {
        let r = run(r#"func main() { let s = "hi"; s.bogus() }"#);
        assert_eq!(r.status, EvalStatus::Error);
        match r.error {
            Some(EvalError::UnknownMethod { ref method, .. }) => {
                assert_eq!(method, "bogus");
            }
            other => panic!("expected UnknownMethod, got {:?}", other),
        }
    }

    #[test]
    fn test_str_method_on_int_errors() {
        // Int 没有 len 方法
        let r = run("func main() { let x = 42; x.len() }");
        assert_eq!(r.status, EvalStatus::Error);
        match r.error {
            Some(EvalError::UnknownMethod { ref receiver_type, ref method }) => {
                assert_eq!(receiver_type, "int");
                assert_eq!(method, "len");
            }
            other => panic!("expected UnknownMethod, got {:?}", other),
        }
    }
}
