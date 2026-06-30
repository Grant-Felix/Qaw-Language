//! qawfmt 集成测试
//!
//! 通过 std::process::Command 直接调用 target/debug/qawfmt 二进制。
//! 与 tests/qawpm.rs 保持同一种风格。

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn qawfmt_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("qawfmt");
    p
}

fn workspace_tmp(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    p.push(format!("qawfmt-it-{name}-{nanos}"));
    fs::create_dir_all(&p).expect("create temp dir");
    p
}

fn write_file(dir: &PathBuf, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).expect("write file");
    path
}

fn run_qawfmt(args: &[&str]) -> std::process::Output {
    Command::new(qawfmt_bin())
        .args(args)
        .output()
        .expect("run qawfmt")
}

#[test]
fn help_exits_zero_and_lists_options() {
    let _tmp = workspace_tmp("help");
    let out = run_qawfmt(&["--help"]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("qawfmt"), "missing tool name; got: {s}");
    assert!(s.contains("--in-place"), "missing --in-place; got: {s}");
    assert!(s.contains("--diff"), "missing --diff; got: {s}");
    assert!(s.contains("--check"), "missing --check; got: {s}");
    assert!(s.contains("--version"), "missing --version; got: {s}");
}

#[test]
fn version_prints_version_string() {
    let _tmp = workspace_tmp("version");
    let out = run_qawfmt(&["--version"]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("qawfmt"), "got: {s}");
    assert!(s.contains("0.11"), "got: {s}");
}

#[test]
fn stdout_mode_strips_trailing_whitespace_and_collapses_blanks() {
    let tmp = workspace_tmp("stdout");
    let f = write_file(
        &tmp,
        "demo.qaw",
        "func main() {   \n\n\n\n    print(\"hi\");\n}\n",
    );
    let out = run_qawfmt(&[f.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("func main() {"), "stdout={s}");
    assert!(s.contains("    print(\"hi\");"), "stdout={s}");
    assert!(!s.contains("   \n"), "应去除行尾空白；stdout={s}");
    // 三行或多行空行合并为单个空行
    assert!(
        !s.contains("\n\n\n"),
        "应合并连续空行；stdout={:?}",
        s
    );
}

#[test]
fn stdout_mode_does_not_touch_source_file() {
    let tmp = workspace_tmp("stdout-no-mut");
    let original = "foo   \n\n\n\nbar\n";
    let f = write_file(&tmp, "demo.qaw", original);
    let out = run_qawfmt(&[f.to_str().unwrap()]);
    assert!(out.status.success());
    let after = fs::read_to_string(&f).unwrap();
    assert_eq!(
        after, original,
        "stdout 模式不应修改源文件"
    );
}

#[test]
fn in_place_overwrites_file() {
    let tmp = workspace_tmp("inplace");
    let f = write_file(&tmp, "demo.qaw", "a   \n\n\n\nb\n");
    let out = run_qawfmt(&["-i", f.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = fs::read_to_string(&f).unwrap();
    assert_eq!(body, "a\n\nb\n", "in-place 后文件内容错误");
}

#[test]
fn in_place_long_flag_works() {
    let tmp = workspace_tmp("inplace-long");
    let f = write_file(&tmp, "demo.qaw", "a   \nb   \n");
    let out = run_qawfmt(&["--in-place", f.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = fs::read_to_string(&f).unwrap();
    assert_eq!(body, "a\nb\n");
}

#[test]
fn check_exits_one_when_formatting_needed() {
    let tmp = workspace_tmp("check-needed");
    let f = write_file(&tmp, "demo.qaw", "a   \n\n\nb\n");
    let out = run_qawfmt(&["--check", f.to_str().unwrap()]);
    assert!(!out.status.success(), "应 exit 1");
    assert_eq!(out.status.code(), Some(1));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("需要格式化"), "got: {s}");
}

#[test]
fn check_exits_zero_when_already_formatted() {
    let tmp = workspace_tmp("check-ok");
    let f = write_file(&tmp, "demo.qaw", "a\nb\n\nc\n");
    let out = run_qawfmt(&["--check", f.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("无需格式化"), "got: {s}");
}

#[test]
fn diff_shows_unified_diff_and_does_not_modify() {
    let tmp = workspace_tmp("diff");
    let original = "foo   \nbar\n\n\nbaz\n";
    let f = write_file(&tmp, "demo.qaw", original);
    let out = run_qawfmt(&["-d", f.to_str().unwrap()]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("--- "), "missing --- header");
    assert!(s.contains("+++ "), "missing +++ header");
    assert!(s.contains("@@ "), "missing hunk header");
    // file unchanged
    let after = fs::read_to_string(&f).unwrap();
    assert_eq!(after, original, "diff 模式不应修改文件");
}

#[test]
fn diff_long_flag_works() {
    let tmp = workspace_tmp("diff-long");
    let f = write_file(&tmp, "demo.qaw", "a  \nb\n");
    let out = run_qawfmt(&["--diff", f.to_str().unwrap()]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("-a"), "got: {s}");
    assert!(s.contains("+a"), "got: {s}");
}

#[test]
fn diff_silent_when_already_formatted() {
    let tmp = workspace_tmp("diff-silent");
    let f = write_file(&tmp, "demo.qaw", "a\nb\n");
    let out = run_qawfmt(&["-d", f.to_str().unwrap()]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert_eq!(s, "", "已格式化的文件不应产生 diff 输出");
}

#[test]
fn missing_file_arg_exits_one() {
    let out = run_qawfmt(&[]);
    assert!(!out.status.success());
}

#[test]
fn nonexistent_file_exits_one() {
    let _tmp = workspace_tmp("nonexistent");
    let out = run_qawfmt(&["/nonexistent/path/to/file.qaw"]);
    assert!(!out.status.success());
}

#[test]
fn unknown_flag_exits_one() {
    let out = run_qawfmt(&["--bogus-flag"]);
    assert!(!out.status.success());
}

#[test]
fn qawpm_demo_main_qaw_is_formatted_correctly() {
    // examples/qawpm-demo/src/main.qaw 由另一 subagent 提交，目前缺少
    // 文件末尾的换行符。本测试断言 qawfmt -i 后文件被修正且第二次格式化
    // 是幂等的。
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("examples");
    p.push("qawpm-demo");
    p.push("src");
    p.push("main.qaw");

    // 复制到临时目录，避免污染仓库文件
    let tmp = workspace_tmp("qawpm-demo");
    let dst = tmp.join("main.qaw");
    let body = fs::read_to_string(&p).expect("read qawpm-demo main.qaw");
    fs::write(&dst, &body).expect("write copy");

    // 1) --check 在初次状态下应该返回非零
    let out = run_qawfmt(&["--check", dst.to_str().unwrap()]);
    assert!(
        !out.status.success(),
        "qawpm-demo main.qaw 当前缺少末尾换行；check 应该失败"
    );

    // 2) -i 后，文件应以单个 \\n 结束
    let out = run_qawfmt(&["-i", dst.to_str().unwrap()]);
    assert!(out.status.success());
    let after = fs::read_to_string(&dst).unwrap();
    assert!(after.ends_with('\n'), "格式化后必须以 \\n 结尾");
    assert!(
        !after.ends_with("\n\n"),
        "不应有多个末尾换行；content={after:?}"
    );

    // 3) 再次 --check 应该通过（幂等性）
    let out = run_qawfmt(&["--check", dst.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "二次格式化后 --check 必须通过；stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn idempotent_on_real_example() {
    let tmp = workspace_tmp("idempotent");
    let f = write_file(&tmp, "demo.qaw", "a   \n\n\n\nb  \nc\n");
    let out1 = run_qawfmt(&["-i", f.to_str().unwrap()]);
    assert!(out1.status.success());
    let once = fs::read_to_string(&f).unwrap();
    let out2 = run_qawfmt(&["-i", f.to_str().unwrap()]);
    assert!(out2.status.success());
    let twice = fs::read_to_string(&f).unwrap();
    assert_eq!(once, twice, "格式化必须是幂等的");
}
