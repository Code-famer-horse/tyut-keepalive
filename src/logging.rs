use anyhow::Result;
use chrono::Local;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const MAX_LOG_BYTES: u64 = 512 * 1024;

static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub(crate) fn init(path: PathBuf) {
    let _ = LOG_PATH.set(path);
}

pub(crate) fn log(message: impl AsRef<str>) {
    let line = format!(
        "[{}] {}",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        message.as_ref()
    );
    println!("{line}");
    if let Some(path) = LOG_PATH.get() {
        if let Err(error) = append(path, &line) {
            eprintln!("写入日志失败：{error:#}");
        }
    }
}

fn append(path: &Path, line: &str) -> Result<()> {
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.len() > MAX_LOG_BYTES {
            let backup = path.with_extension("log.1");
            let _ = fs::remove_file(&backup);
            fs::rename(path, backup)?;
        }
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub(crate) fn tail(limit: usize) -> String {
    let Some(path) = LOG_PATH.get() else {
        return String::new();
    };
    let Ok(text) = fs::read_to_string(path) else {
        return String::new();
    };
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(limit);
    lines[start..].join("\n")
}
