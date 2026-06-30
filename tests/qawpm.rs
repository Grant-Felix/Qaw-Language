use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn qawpm_bin() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    p.push("debug");
    p.push("qawpm");
    p
}

fn workspace_tmp(name: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    p.push(format!("qawpm-it-{name}-{nanos}"));
    fs::create_dir_all(&p).expect("create temp dir");
    p
}

fn run_qawpm(cwd: &PathBuf, args: &[&str]) -> std::process::Output {
    Command::new(qawpm_bin())
        .args(args)
        .current_dir(cwd)
        .env("QAW_HOME", cwd.join("home"))
        .output()
        .expect("run qawpm")
}

#[test]
fn help_exits_zero_and_lists_commands() {
    let tmp = workspace_tmp("help");
    let out = run_qawpm(&tmp, &["--help"]);
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("qawpm"));
    assert!(s.contains("init"));
    assert!(s.contains("add"));
    assert!(s.contains("install"));
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn version_prints_version_string() {
    let tmp = workspace_tmp("version");
    let out = run_qawpm(&tmp, &["--version"]);
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("qawpm"), "got: {s}");
    assert!(s.contains("0.11"), "got: {s}");
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn init_creates_toml_and_src_dir() {
    let tmp = workspace_tmp("init");
    let out = run_qawpm(&tmp, &["init"]);
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));

    let toml = tmp.join("Qaw.toml");
    assert!(toml.exists(), "Qaw.toml not created");
    let src = tmp.join("src");
    assert!(src.is_dir(), "src/ not created");
    let main_qaw = src.join("main.qaw");
    assert!(main_qaw.exists(), "src/main.qaw not created");

    let body = fs::read_to_string(&toml).unwrap();
    assert!(body.contains("[package]"));
    assert!(body.contains("[dependencies]"));
    assert!(body.contains("name = \""));
    assert!(body.contains("version = \""));
    assert!(body.contains("edition = \"2024\""));
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn add_appends_dependency_to_toml() {
    let tmp = workspace_tmp("add");
    assert!(run_qawpm(&tmp, &["init"]).status.success());

    let out = run_qawpm(&tmp, &["add", "qaw_std", "1.0"]);
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));

    let body = fs::read_to_string(tmp.join("Qaw.toml")).unwrap();
    assert!(body.contains("qaw_std = \"1.0\""), "body={body}");

    run_qawpm(&tmp, &["add", "qaw_http", "2.1"]);
    let body = fs::read_to_string(tmp.join("Qaw.toml")).unwrap();
    assert!(body.contains("qaw_std = \"1.0\""));
    assert!(body.contains("qaw_http = \"2.1\""));
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn install_creates_lockfile_and_cache_manifest() {
    let tmp = workspace_tmp("install");
    assert!(run_qawpm(&tmp, &["init"]).status.success());
    assert!(run_qawpm(&tmp, &["add", "qaw_std", "1.0"]).status.success());

    let out = run_qawpm(&tmp, &["install"]);
    assert!(out.status.success(), "stderr={}", String::from_utf8_lossy(&out.stderr));

    let lock = tmp.join("Qaw.lock");
    assert!(lock.exists(), "Qaw.lock not created");
    let lock_body = fs::read_to_string(&lock).unwrap();
    assert!(lock_body.contains("[[package]]"));
    assert!(lock_body.contains("name = \"qaw_std\""));
    assert!(lock_body.contains("version = \"1.0\""));
    assert!(lock_body.contains("source = \"registry+https://qaw-lang.org\""));

    let cache_manifest = tmp.join("home").join(".qaw").join("cache").join("manifest.txt");
    assert!(cache_manifest.exists(), "cache manifest not created at {cache_manifest:?}");
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn install_fails_when_no_qaw_toml() {
    let tmp = workspace_tmp("install-err");
    let out = run_qawpm(&tmp, &["install"]);
    assert!(!out.status.success(), "install should fail without Qaw.toml");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Qaw.toml"), "stderr={stderr}");
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn add_requires_existing_toml() {
    let tmp = workspace_tmp("add-err");
    let out = run_qawpm(&tmp, &["add", "foo", "1.0"]);
    assert!(!out.status.success(), "add should fail without Qaw.toml");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Qaw.toml"), "stderr={stderr}");
    fs::remove_dir_all(&tmp).ok();
}

#[test]
fn init_refuses_to_overwrite_existing_toml() {
    let tmp = workspace_tmp("init-overwrite");
    assert!(run_qawpm(&tmp, &["init"]).status.success());
    let out = run_qawpm(&tmp, &["init"]);
    assert!(!out.status.success(), "second init should refuse");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("已存在"), "stderr={stderr}");
    fs::remove_dir_all(&tmp).ok();
}