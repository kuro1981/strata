//! 統合テスト共通ヘルパ。バイナリは `CARGO_BIN_EXE_strata-cli`(cargo が test 時に
//! 埋める)で解決する。追加依存なし(std::process::Command のみ)。
// 複数のテストバイナリから include されるため、バイナリによっては未使用の
// ヘルパが出る(test 共有モジュールの定番)。dead_code 警告は抑止する。
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// テスト対象バイナリへの絶対パス。
pub fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_strata-cli")
}

/// リポジトリルート(crates/strata-cli の2つ上)。fixture 参照に使う。
/// `mod common` は各統合テストバイナリ(`tests/*.rs` はそれぞれ独立クレート)に
/// 個別コンパイルされるため、fixture 参照を使わないバイナリ(例: workspace_cli.rs、
/// 合成ワークスペースのみを扱い docs/ の固定 fixture には依存しない)ではこの関数が
/// 未使用になる。共有ヘルパファイルの性質上そちらは正当な未使用なので許容する。
#[allow(dead_code)]
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("repo root exists")
}

/// テストごとに一意な一時ディレクトリを作る。Drop で削除する。
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(label: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir()
            .join(format!("strata-cli-test-{}-{}-{}", std::process::id(), label, n));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        TempDir(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// バイナリを引数付きで実行する。
pub fn run(args: &[&str]) -> Output {
    Command::new(bin()).args(args).output().expect("failed to spawn strata-cli")
}

pub fn exit_code(out: &Output) -> i32 {
    out.status.code().expect("process terminated by signal")
}

pub fn stdout_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

pub fn stderr_str(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}
