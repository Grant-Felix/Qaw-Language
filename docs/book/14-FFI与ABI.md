# FFI 与 ABI

[← 返回目录](../1-妖文编程语言从入门到精通.md)

---


## 14.1 与 C 互操作

```yao
@extern "C" {
    fn abs(x: int) -> int;
    fn strlen(s: *const byte) -> usize;
}
```

## 14.2 内存布局

```yao
#[repr(C)]
struct Point {
    x: f64,    // offset 0
    y: f64,    // offset 8
}
```

## 14.3 字符串转换

```yao
@unsafe {
    let c_str = "hello".as_ptr();        // *const byte
    let buf = malloc(100) as *mut u8;
}
```

## 14.4 Python 互操作

```yao
@dynamic
func compute() -> any {
    var np = importPython("numpy");
    var arr = np.array([1, 2, 3]);
    return arr.sum();
}
```

## 14.5 WASM 互操作

```yao
#[wasm_bindgen]
extern "js" {
    fn alert(s: &str);
}

#[wasm_bindgen]
method greet(name: &str) {
    alert(&format!("Hello, {}!", name));
}
```

---
