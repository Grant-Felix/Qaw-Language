//! qawtest 集成测试
//!
//! 通过 std::process::Command 直接调用 target/debug/qawtest 二进制。
//! 风格与 tests/qawfmt.rs / tests/qawpm.rs 一致。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn qawtest_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("qawtest");
    p
}

fn workspace_tmp(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    p.push(format!("qawtest-it-{name}-{nanos}"));
    fs::create_dir_all(&p).expect("create temp dir");
    p
}

fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).expect("write file");
    path
}

fn run_qawtest(args: &[&str]) -> std::process::Output {
    Command::new(qawtest_bin())
        .args(args)
        .output()
        .expect("run qawtest")
}

fn run_qawtest_in(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(qawtest_bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run qawtest")
}

// ============ CLI 基本测试 ============

#[test]
fn help_exits_zero_and_lists_options() {
    let _tmp = workspace_tmp("help");
    let out = run_qawtest(&["--help"]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("qawtest"), "missing tool name; got: {s}");
    assert!(s.contains("--filter"), "missing --filter; got: {s}");
    assert!(s.contains("--help"), "missing --help; got: {s}");
    assert!(s.contains("--version"), "missing --version; got: {s}");
}

#[test]
fn version_prints_version_string() {
    let _tmp = workspace_tmp("version");
    let out = run_qawtest(&["--version"]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("qawtest"), "got: {s}");
    assert!(s.contains("0.18"), "got: {s}");
}

#[test]
fn unknown_flag_exits_one() {
    let out = run_qawtest(&["--bogus-flag"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("未知选项") || stderr.contains("用法"),
        "stderr={stderr}"
    );
}

#[test]
fn filter_without_value_exits_one() {
    let out = run_qawtest(&["--filter"]);
    assert!(!out.status.success());
}

#[test]
fn nonexistent_path_exits_one() {
    let out = run_qawtest(&["/nonexistent/path/should/not/exist"]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("路径不存在") || stderr.contains("不存在"),
        "stderr={stderr}"
    );
}

// ============ 文件发现 ============

#[test]
fn hello_qaw_yields_zero_tests() {
    // examples/hello.qaw 不含 // @test，应输出 0 tests
    let out = run_qawtest(&["examples/hello.qaw"]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        s.contains("running 0 tests"),
        "应输出 'running 0 tests'；got: {s}"
    );
    assert!(
        s.contains("0 passed; 0 failed; 0 ignored"),
        "应输出通过 / 失败 / 忽略计数；got: {s}"
    );
}

#[test]
fn scan_directory_finds_qaw_files() {
    // 把测试文件放进临时目录，确认 qawtest 会扫描整个目录
    let tmp = workspace_tmp("scan");
    write_file(
        &tmp,
        "a.qaw",
        "\
func helper() -> int { return 1; }

// @test
func test_a() {
    let r = helper();
    if r != 1 { print(\"FAIL\"); }
}
",
    );
    write_file(
        &tmp,
        "b.qaw",
        "\
// @test
func test_b() {
    let x = 1 + 1;
    if x != 2 { print(\"FAIL\"); }
}
",
    );
    write_file(&tmp, "c.txt", "不是 qaw 文件");

    let out = run_qawtest(&[tmp.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("running 2 tests"), "got: {s}");
    assert!(s.contains("test test_a ... ok"), "got: {s}");
    assert!(s.contains("test test_b ... ok"), "got: {s}");
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn scan_current_directory_when_no_target() {
    // qawtest 不带参数应扫描当前目录（这里用一个临时子目录作为 cwd）
    let tmp = workspace_tmp("cwd");
    write_file(
        &tmp,
        "demo.qaw",
        "\
// @test
func test_x() {
    let y = 2 + 2;
    if y != 4 { print(\"FAIL\"); }
}
",
    );

    let out = run_qawtest_in(&tmp, &[]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("running 1 tests"), "got: {s}");
    assert!(s.contains("test test_x ... ok"), "got: {s}");
    fs::remove_dir_all(&tmp).ok();
}

// ============ 测试发现 ============

#[test]
fn finds_simple_test_in_single_file() {
    let tmp = workspace_tmp("simple");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "\
func add(a: int, b: int) -> int { return a + b; }

// @test
func test_add() {
    let r = add(2, 3);
    if r != 5 { print(\"FAIL\"); }
}
",
    );
    let out = run_qawtest(&[f.to_str().unwrap()]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("running 1 tests"), "got: {s}");
    assert!(s.contains("test test_add ... ok"), "got: {s}");
    assert!(s.contains("1 passed; 0 failed; 0 ignored"), "got: {s}");
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn finds_multiple_tests() {
    let tmp = workspace_tmp("multi");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "\
func add(a: int, b: int) -> int { return a + b; }
func sub(a: int, b: int) -> int { return a - b; }

// @test
func test_a() {
    if add(1, 2) != 3 { print(\"FAIL\"); }
}

// @test
func test_b() {
    if sub(10, 4) != 6 { print(\"FAIL\"); }
}

// @test
func test_c() {
    if add(0, 0) != 0 { print(\"FAIL\"); }
}
",
    );
    let out = run_qawtest(&[f.to_str().unwrap()]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("running 3 tests"), "got: {s}");
    assert!(s.contains("test test_a ... ok"), "got: {s}");
    assert!(s.contains("test test_b ... ok"), "got: {s}");
    assert!(s.contains("test test_c ... ok"), "got: {s}");
    assert!(s.contains("3 passed; 0 failed; 0 ignored"), "got: {s}");
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn fails_when_assertion_violated() {
    let tmp = workspace_tmp("fail");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "\
func div(a: int, b: int) -> int { return a / b; }

// @test
func test_div() {
    let r = div(1, 0);
}
",
    );
    let out = run_qawtest(&[f.to_str().unwrap()]);
    assert!(!out.status.success(), "测试除零应失败");
    assert_eq!(out.status.code(), Some(1));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("running 1 tests"), "got: {s}");
    assert!(
        s.contains("test test_div ... FAILED"),
        "got: {s}"
    );
    assert!(s.contains("0 passed; 1 failed; 0 ignored"), "got: {s}");
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn mixed_pass_and_fail_in_same_file() {
    let tmp = workspace_tmp("mixed");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "\
// @test
func test_pass() {
    if 1 + 1 != 2 { print(\"FAIL\"); }
}

// @test
func test_fail() {
    let x = 1 / 0;
}
",
    );
    let out = run_qawtest(&[f.to_str().unwrap()]);
    assert!(!out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("test test_pass ... ok"), "got: {s}");
    assert!(s.contains("test test_fail ... FAILED"), "got: {s}");
    assert!(s.contains("1 passed; 1 failed; 0 ignored"), "got: {s}");
    fs::remove_dir_all(&tmp).ok();
}

// ============ --filter 测试 ============

#[test]
fn filter_runs_only_matching_tests() {
    let tmp = workspace_tmp("filter");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "\
// @test
func test_add_one() {
    if 1 + 1 != 2 { print(\"FAIL\"); }
}

// @test
func test_sub_two() {
    if 5 - 2 != 3 { print(\"FAIL\"); }
}

// @test
func test_add_two() {
    if 2 + 2 != 4 { print(\"FAIL\"); }
}
",
    );
    let out = run_qawtest(&[f.to_str().unwrap(), "--filter", "add"]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("running 2 tests"), "got: {s}");
    assert!(s.contains("test test_add_one ... ok"), "got: {s}");
    assert!(s.contains("test test_add_two ... ok"), "got: {s}");
    assert!(!s.contains("test_sub_two"), "filter 应排除 test_sub_two；got: {s}");
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn filter_no_match_yields_zero() {
    let tmp = workspace_tmp("filter-none");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "\
// @test
func test_a() {}
",
    );
    let out = run_qawtest(&[f.to_str().unwrap(), "--filter", "nonexistent"]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("running 0 tests"), "got: {s}");
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn filter_matches_custom_name() {
    let tmp = workspace_tmp("filter-custom");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "\
// @test name=\"加法测试\"
func test_a() {
    if 1 + 1 != 2 { print(\"FAIL\"); }
}

// @test
func test_b() {}
",
    );
    let out = run_qawtest(&[f.to_str().unwrap(), "--filter", "加法"]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("running 1 tests"), "got: {s}");
    assert!(s.contains("test 加法测试 ... ok"), "got: {s}");
    fs::remove_dir_all(&tmp).ok();
}

// ============ main 剥离测试 ============

#[test]
fn main_is_not_executed_when_tests_run() {
    // 即使文件里有 main，qawtest 不应执行 main
    // 通过 print 的 side effect 检测：MAIN-SHOULD-NOT-RUN 不应出现在 stdout
    let tmp = workspace_tmp("main-strip");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "\
func helper() -> int { return 42; }

func main() {
    print(\"MAIN-SHOULD-NOT-RUN\");
}

// @test
func test_helper() {
    let r = helper();
    if r != 42 { print(\"FAIL\"); }
}
",
    );
    let out = run_qawtest(&[f.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stdout.contains("MAIN-SHOULD-NOT-RUN"),
        "main 不应被执行；stdout={stdout}"
    );
    assert!(
        !stderr.contains("MAIN-SHOULD-NOT-RUN"),
        "main 不应被执行；stderr={stderr}"
    );
    assert!(stdout.contains("test test_helper ... ok"), "got: {stdout}");
    fs::remove_dir_all(&tmp).ok();
}

// ============ 输出格式 ============

#[test]
fn output_format_matches_rust_test_style() {
    let tmp = workspace_tmp("format");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "\
// @test
func test_one() {}
",
    );
    let out = run_qawtest(&[f.to_str().unwrap()]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    // 行格式：test test_one ... ok
    assert!(
        s.contains("running 1 tests\n"),
        "应有 'running 1 tests' 单独一行；got: {s:?}"
    );
    assert!(
        s.contains("test test_one ... ok\n"),
        "应有 'test test_one ... ok' 一行；got: {s:?}"
    );
    // 空行
    assert!(
        s.contains("\n\ntest result:"),
        "结果行前应有空行；got: {s:?}"
    );
    assert!(
        s.contains("test result: 1 passed; 0 failed; 0 ignored"),
        "结果行格式；got: {s:?}"
    );
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn test_panic_messages_are_included_in_output() {
    // 让一个测试触发 EvalError，确认错误信息出现在输出里
    let tmp = workspace_tmp("msg");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "\
func div(a: int, b: int) -> int { return a / b; }

// @test
func test_div_zero() {
    let r = div(10, 0);
}
",
    );
    let out = run_qawtest(&[f.to_str().unwrap()]);
    assert!(!out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("FAILED"), "got: {s}");
    assert!(
        s.contains("除数为零") || s.contains("零"),
        "错误信息应包含除零；got: {s}"
    );
    fs::remove_dir_all(&tmp).ok();
}