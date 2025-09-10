/// 缓存key
#[derive(strum_macros::Display)]
pub enum CacheKey {
    /// 用户Token，用于本系统登录
    /// 0: 用户Token
    #[strum(to_string = "oag:user:token:{0}")]
    UserToken(String),
}
