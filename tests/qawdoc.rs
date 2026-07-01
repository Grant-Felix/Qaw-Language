//! qawdoc 集成测试
//!
//! 通过 std::process::Command 调用 target/debug/qawdoc 二进制。
//! 风格与 tests/qawfmt.rs / tests/qawpm.rs 一致。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn qawdoc_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("qawdoc");
    p
}

fn workspace_tmp(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    p.push(format!("qawdoc-it-{name}-{nanos}"));
    fs::create_dir_all(&p).expect("create temp dir");
    p
}

fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).expect("write file");
    path
}

fn run_qawdoc(args: &[&str]) -> std::process::Output {
    Command::new(qawdoc_bin())
        .args(args)
        .output()
        .expect("run qawdoc")
}

#[test]
fn help_exits_zero_and_lists_options() {
    let _tmp = workspace_tmp("help");
    let out = run_qawdoc(&["--help"]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("qawdoc"), "missing tool name; got: {s}");
    assert!(s.contains("--version"), "missing --version; got: {s}");
    assert!(s.contains("-o"), "missing -o; got: {s}");
    assert!(s.contains("--recursive"), "missing --recursive; got: {s}");
}

#[test]
fn version_prints_version_string() {
    let _tmp = workspace_tmp("version");
    let out = run_qawdoc(&["--version"]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("qawdoc"), "got: {s}");
    assert!(s.contains("0.18"), "got: {s}");
}

#[test]
fn missing_file_arg_exits_one() {
    let out = run_qawdoc(&[]);
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn unknown_flag_exits_one() {
    let out = run_qawdoc(&["--bogus-flag"]);
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn hello_example_produces_non_empty_output() {
    // 验收要求 1：examples/hello.qaw 应输出非空 Markdown
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("examples");
    p.push("hello.qaw");
    let out = run_qawdoc(&[p.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(!s.is_empty(), "expected non-empty output");
    assert!(s.starts_with("# hello"), "expected '# hello' header; got: {s}");
}

#[test]
fn extracts_doc_comments_from_real_file() {
    let tmp = workspace_tmp("real");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "\
/// 主程序入口
func main() {
    print(\"hi\");
}

/// 圆周率
const PI = 3;
",
    );
    let out = run_qawdoc(&[f.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("# demo"), "got: {s}");
    assert!(s.contains("## Functions"), "got: {s}");
    assert!(s.contains("### main"), "got: {s}");
    assert!(s.contains("主程序入口"), "got: {s}");
    assert!(s.contains("## Constants"), "got: {s}");
    assert!(s.contains("### PI"), "got: {s}");
    assert!(s.contains("圆周率"), "got: {s}");
}

#[test]
fn writes_to_output_file_when_o_given() {
    let tmp = workspace_tmp("out");
    let src = write_file(&tmp, "src.qaw", "/// doc\nfunc main() {}\n");
    let dst = tmp.join("out.md");
    let out = run_qawdoc(&["-o", dst.to_str().unwrap(), src.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = fs::read_to_string(&dst).expect("read output");
    assert!(body.contains("# src"));
    assert!(body.contains("### main"));
    assert!(body.contains("doc"));
}

#[test]
fn nonexistent_file_exits_one() {
    let out = run_qawdoc(&["/nonexistent/path/file.qaw"]);
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("错误"), "expected error on stderr; got: {err}");
}

#[test]
fn orphan_doc_comments_are_dropped() {
    let tmp = workspace_tmp("orphan");
    let f = write_file(
        &tmp,
        "orphan.qaw",
        "\
func main() {}

/// 这条 doc 后面没有声明，应当被丢弃
/// 第二行
",
    );
    let out = run_qawdoc(&[f.to_str().unwrap()]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(!s.contains("应当被丢弃"), "orphan doc leaked into output: {s}");
    assert!(!s.contains("### main"), "main has no doc; should not appear: {s}");
    // 输出应只包含标题
    assert_eq!(s.trim(), "# orphan");
}

#[test]
fn paragraph_break_preserved_in_doc() {
    let tmp = workspace_tmp("para");
    let f = write_file(
        &tmp,
        "para.qaw",
        "\
/// 第一段
///
/// 第二段
func main() {}
",
    );
    let out = run_qawdoc(&[f.to_str().unwrap()]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("第一段"), "got: {s}");
    assert!(s.contains("第二段"), "got: {s}");
    // 段间应为两个换行（Markdown 段落分隔）
    assert!(s.contains("第一段\n\n第二段"), "paragraph break missing; got: {s}");
}