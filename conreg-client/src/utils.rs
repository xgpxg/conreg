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


#[cfg(feature = "tracing")]
static TRACING_HAS_INIT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(feature = "tracing")]
pub(crate) fn init_log() {
    if TRACING_HAS_INIT.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_level(true)
        .with_ansi(true)
        .with_line_number(true)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%Y-%m-%d %H:%M:%S.%3f".to_string(),
        ))
        .compact()
        .init();
    TRACING_HAS_INIT.store(true, std::sync::atomic::Ordering::Relaxed);
}
