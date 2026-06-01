use std::fmt::Display;

pub mod api;
mod user;

pub use user::{
    append_user_permissions_and_sync, check_ns_permission, clean_ns_permissions_and_sync,
    create_user, delete_user, get_user_permissions, update_user,
};

#[allow(clippy::enum_variant_names)]
pub(crate) enum UserPermission {
    ReadWritePublicNs,
    #[allow(unused)]
    ReadNs(String),
    #[allow(unused)]
    WriteNs(String),
    ReadWriteNs(String),
}

impl Display for UserPermission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserPermission::ReadWritePublicNs => write!(f, "rw:ns:public"),
            UserPermission::ReadNs(ns) => write!(f, "r:ns:{}", ns),
            UserPermission::WriteNs(ns) => write!(f, "w:ns:{}", ns),
            UserPermission::ReadWriteNs(ns) => write!(f, "rw:ns:{}", ns),
        }
    }
}

impl UserPermission {}
