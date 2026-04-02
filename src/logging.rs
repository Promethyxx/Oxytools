use std::fs::OpenOptions;
use std::io::Write;

fn log_path() -> std::path::PathBuf {
    let dir = std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("config");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("oxyon.log")
}

pub fn log_entry(level: &str, message: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("worker");
    let log_line = format!("[{}] [{}] [{}] {}\n", timestamp, level, thread_name, message);
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path())
    {
        let _ = file.write_all(log_line.as_bytes());
    }
}

pub fn log_info(msg: &str)  { log_entry("INFO",  msg); }
pub fn log_warn(msg: &str)  { log_entry("WARN",  msg); }
pub fn log_error(msg: &str) { log_entry("ERROR", msg); }