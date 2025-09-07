use env_logger::WriteStyle;

/// 获取当前进程名称
pub(crate) fn current_process_name() -> String {
    std::env::args()
        .next()
        .as_ref()
        .map(std::path::Path::new)
        .and_then(std::path::Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .map(String::from)
        .unwrap_or_else(|| "unknown".to_string())
}

pub(crate) fn init_log() {
    use std::io::Write;
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .format(|buf, record| {
            let level = record.level().as_str();
            writeln!(
                buf,
                "[{}][{}] - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                level,
                record.args()
            )
        })
        .write_style(WriteStyle::Always)
        .init();
}
