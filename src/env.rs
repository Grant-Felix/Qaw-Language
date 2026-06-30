//! 执行环境（变量作用域）

use std::collections::HashMap;
use crate::value::Value;

/// 环境：变量绑定 + 父作用域
#[derive(Debug, Clone)]
pub struct Env {
    bindings: HashMap<String, Value>,
    pub parent: Option<Box<Env>>,
}

impl Env {
    pub fn new() -> Self {
        Env {
            bindings: HashMap::new(),
            parent: None,
        }
    }

    pub fn child(parent: Env) -> Self {
        Env {
            bindings: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    /// 定义新变量（遮蔽父作用域同名变量）
    pub fn define(&mut self, name: &str, value: Value) {
        self.bindings.insert(name.to_string(), value);
    }

    /// 设置已存在变量（向父作用域查找）
    pub fn set(&mut self, name: &str, value: Value) -> bool {
        if self.bindings.contains_key(name) {
            self.bindings.insert(name.to_string(), value);
            return true;
        }
        if let Some(p) = &mut self.parent {
            return p.set(name, value);
        }
        false
    }

    /// 获取变量（向上查找）
    pub fn get(&self, name: &str) -> Value {
        if let Some(v) = self.bindings.get(name) {
            return v.clone();
        }
        if let Some(p) = &self.parent {
            return p.get(name);
        }
        Value::Nil
    }

    /// 是否定义（向上查找）
    pub fn has(&self, name: &str) -> bool {
        if self.bindings.contains_key(name) {
            return true;
        }
        if let Some(p) = &self.parent {
            return p.has(name);
        }
        false
    }
}

impl Default for Env {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::*;

    #[test]
    fn test_root() {
        let e = Env::new();
        assert!(!e.has("x"));
    }

    #[test]
    fn test_define_get() {
        let mut e = Env::new();
        e.define("x", val_int(42));
        assert_eq!(e.get("x").to_int(), 42);
    }

    #[test]
    fn test_set_existing() {
        let mut e = Env::new();
        e.define("x", val_int(1));
        assert!(e.set("x", val_int(2)));
        assert_eq!(e.get("x").to_int(), 2);
    }

    #[test]
    fn test_set_undefined() {
        let mut e = Env::new();
        assert!(!e.set("x", val_int(1)));
    }

    #[test]
    fn test_child_scope() {
        let mut root = Env::new();
        root.define("x", val_int(1));
        let mut child = Env::child(root);
        assert_eq!(child.get("x").to_int(), 1);
        child.define("x", val_int(2));
        assert_eq!(child.get("x").to_int(), 2);
    }
}
