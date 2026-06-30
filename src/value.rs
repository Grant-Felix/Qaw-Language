//! Qaw 运行时值

use std::fmt;

/// 值类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueKind {
    Int,
    Float,
    Bool,
    String,
    Nil,
}

impl ValueKind {
}

// 值（tagged union）
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Nil,
}

// 提供字符串访问
impl Value {
}

impl Value {
    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Int(_) => ValueKind::Int,
            Value::Float(_) => ValueKind::Float,
            Value::Bool(_) => ValueKind::Bool,
            Value::String(_) => ValueKind::String,
            Value::Nil => ValueKind::Nil,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
        }
    }

    pub fn to_int(&self) -> i64 {
        match self {
            Value::Int(n) => *n,
            Value::Float(f) => *f as i64,
            Value::Bool(b) => if *b { 1 } else { 0 },
            Value::String(s) => s.parse().unwrap_or(0),
            Value::Nil => 0,
        }
    }

    pub fn to_float(&self) -> f64 {
        match self {
            Value::Int(n) => *n as f64,
            Value::Float(f) => *f,
            Value::Bool(b) => if *b { 1.0 } else { 0.0 },
            Value::String(s) => s.parse().unwrap_or(0.0),
            Value::Nil => 0.0,
        }
    }

    pub fn to_bool(&self) -> bool {
        self.is_truthy()
    }

    pub fn equal(&self, other: &Value) -> bool {
        if std::mem::discriminant(self) != std::mem::discriminant(other) {
            // 跨类型：尝试数值比较
            if matches!(self.kind(), ValueKind::Int | ValueKind::Float)
                && matches!(other.kind(), ValueKind::Int | ValueKind::Float)
            {
                return self.to_float() == other.to_float();
            }
            return false;
        }
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(v) => write!(f, "{}", v),
            Value::Bool(b) => write!(f, "{}", b),
            Value::String(s) => write!(f, "{}", s),
            Value::Nil => write!(f, "nil"),
        }
    }
}

pub fn val_int(n: i64) -> Value { Value::Int(n) }
pub fn val_float(f: f64) -> Value { Value::Float(f) }
pub fn val_bool(b: bool) -> Value { Value::Bool(b) }
pub fn val_string(s: impl Into<String>) -> Value { Value::String(s.into()) }
pub fn val_nil() -> Value { Value::Nil }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int() {
        let v = val_int(42);
        assert_eq!(v.kind(), ValueKind::Int);
        assert_eq!(v.to_int(), 42);
    }

    #[test]
    fn test_truthy() {
        assert!(!val_nil().is_truthy());
        assert!(!val_bool(false).is_truthy());
        assert!(val_bool(true).is_truthy());
        assert!(!val_int(0).is_truthy());
        assert!(val_int(-1).is_truthy());
        assert!(!val_string("").is_truthy());
    }

    #[test]
    fn test_equal() {
        assert!(val_int(5).equal(&val_int(5)));
        assert!(!val_int(5).equal(&val_int(6)));
        assert!(val_string("hi").equal(&val_string("hi")));
        assert!(val_nil().equal(&val_nil()));
        assert!(val_int(5).equal(&val_float(5.0)));
    }
}
