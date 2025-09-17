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
