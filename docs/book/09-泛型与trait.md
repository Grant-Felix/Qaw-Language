# 泛型与 trait

[← 返回目录](../1-妖文编程语言从入门到精通.md)

---


## 9.1 泛型函数

```yao
func identity<T>(x: T) -> T {
    x
}

let a = identity(42);          // T = int
let b = identity("hello");     // T = &str
```

## 9.2 泛型结构体

```yao
struct Box<T> {
    value: T,
}

impl<T> Box<T> {
    method new(value: T) -> Box<T> {
        Box { value }
    }
    
    method unbox(self) -> T {
        self.value
    }
}
```

## 9.3 trait 定义

```yao
trait Display {
    method fmt(self, f: &mut Formatter) -> Result;
}

trait Greet: Display {        // Greet 要求实现 Display
    method greet(self) -> string;
}
```

## 9.4 trait 实现

```yao
struct Point { x: f64, y: f64 }

impl Display for Point {
    method fmt(self, f: &mut Formatter) -> Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}
```

## 9.5 trait 约束

```yao
func print<T: Display>(x: T) {
    // ...
}

func complex<T, U>(t: T, u: U)
where
    T: Display + Clone,
    U: Debug + Into<string>,
{ ... }
```

## 9.6 派生

```yao
#[derive(Clone, Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}
```

可派生 trait：`Clone`、`Copy`、`Debug`、`Display`、`Default`、`Hash`、`PartialEq`、`Eq`、`PartialOrd`、`Ord`、`Send`、`Sync`。

## 9.7 trait object

```yao
let drawables: [dyn Drawable] = [
    Circle { radius: 1.0 },
    Rectangle { width: 2.0, height: 3.0 },
];

for d in drawables {
    d.draw();    // 动态分派
}
```

---
