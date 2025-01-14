use cargo_play::opt::Opt;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::env;
use std::ffi::OsStr;
use std::io::Result;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output, Stdio};

struct TestRuntime {
    scratch: PathBuf,
}

impl TestRuntime {
    fn new() -> Result<Self> {
        let scratch = Self::create_scratch_dir()?;

        Ok(TestRuntime { scratch })
    }

    fn create_scratch_dir() -> Result<PathBuf> {
        let tmp = env::temp_dir();
        let scratch = tmp.join(Self::random_string());

        if scratch.exists() {
            let _ = std::fs::remove_dir_all(&scratch);
        }

        std::fs::create_dir(&scratch)?;

        Ok(scratch)
    }

    fn random_string() -> String {
        format!(
            "cargo-play-test.{}",
            thread_rng()
                .sample_iter(&Alphanumeric)
                .take(10)
                .collect::<String>()
        )
    }

    fn temp_dir<I: AsRef<Path>>(&self, path: I) -> PathBuf {
        self.scratch.join(path)
    }

    fn run<I: IntoIterator<Item = S> + std::fmt::Debug, S: AsRef<OsStr> + std::fmt::Debug>(
        &self,
        args: I,
    ) -> std::io::Result<StringOutput> {
        let mut play = std::process::Command::new(cargo_play_binary_path());
        play.env("TMP", &self.scratch)
            .env("TMPDIR", &self.scratch)
            .args(args)
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .output()
            .map(From::from)
    }
}

impl Drop for TestRuntime {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.scratch);
    }
}

fn cargo_play_binary_path() -> PathBuf {
    let mut path = env::current_exe().unwrap();
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    let exe = String::from("cargo-play") + env::consts::EXE_SUFFIX;
    path.push(exe);
    path
}

#[derive(Debug)]
struct StringOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl From<std::process::Output> for StringOutput {
    fn from(v: Output) -> Self {
        StringOutput {
            status: v.status,
            stdout: String::from_utf8_lossy(&v.stdout).to_string(),
            stderr: String::from_utf8_lossy(&v.stderr).to_string(),
        }
    }
}

#[test]
fn basic() -> Result<()> {
    let rt = TestRuntime::new()?;
    let output = rt.run(&["fixtures/hello.rs"])?;

    assert_eq!(output.status.code().unwrap(), 0);
    assert_eq!(output.stdout, "Hello World!\n");

    Ok(())
}

#[test]
fn clean() -> Result<()> {
    let rt = TestRuntime::new()?;
    let opt = Opt::with_files(vec!["fixtures/hello.rs"]);
    let path = rt.temp_dir(opt.temp_dirname());
    let canary = path.clone().join("canary");

    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }

    println!("{:?}", path);
    let _ = rt.run(&["fixtures/hello.rs"])?;
    assert!(path.exists());

    std::fs::write(&canary, "I_AM_CANARY")?;

    assert!(canary.exists());
    let _ = rt.run(&["--clean", "fixtures/hello.rs"])?;
    assert!(!canary.exists());

    Ok(())
}

#[test]
fn edition() -> Result<()> {
    let rt = TestRuntime::new()?;

    // default edition is 2018
    let output = rt.run(&["fixtures/edition.rs"])?;
    assert_ne!(output.status.code().unwrap(), 0);

    let output = rt.run(&["--edition", "2018", "fixtures/edition.rs"])?;
    assert_ne!(output.status.code().unwrap(), 0);

    // it should pass in 2015
    let output = rt.run(&["--edition", "2015", "fixtures/edition.rs"])?;
    assert_eq!(output.status.code().unwrap(), 0);

    Ok(())
}

#[test]
fn debug_mode() -> Result<()> {
    let rt = TestRuntime::new()?;

    let opt = Opt::with_files(vec!["fixtures/hello.rs"]);
    let path = rt.temp_dir(opt.temp_dirname());

    let _ = rt.run(&["fixtures/hello.rs"])?;
    assert!(path.join("target").join("debug").exists());
    assert!(!path.join("target").join("release").exists());

    Ok(())
}

#[test]
fn release_mode() -> Result<()> {
    let rt = TestRuntime::new()?;

    let opt = Opt::with_files(vec!["fixtures/hello.rs"]);
    let path = rt.temp_dir(opt.temp_dirname());

    let _ = rt.run(&["--release", "fixtures/hello.rs"])?;
    assert!(!path.join("target").join("debug").exists());
    assert!(path.join("target").join("release").exists());

    Ok(())
}

#[test]
fn cargo_option() -> Result<()> {
    let rt = TestRuntime::new()?;

    let opt = Opt::with_files(vec!["fixtures/hello.rs"]);
    let path = rt.temp_dir(opt.temp_dirname());

    let _ = rt.run(&["--cargo-option=--release", "fixtures/hello.rs"])?;

    assert!(!path.join("target").join("debug").exists());
    assert!(path.join("target").join("release").exists());

    Ok(())
}

#[test]
fn program_args() -> Result<()> {
    let rt = TestRuntime::new()?;

    let output = rt.run(&["fixtures/args.rs", "--", "test"])?;
    assert_eq!(output.stdout, "test\n");

    Ok(())
}

#[test]
fn external_crate() -> Result<()> {
    let rt = TestRuntime::new()?;

    let output = rt.run(&["fixtures/bitflags.rs"])?;
    assert_eq!(output.status.code().unwrap(), 0);

    Ok(())
}
