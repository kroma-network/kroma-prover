use log::{error, info};
use std::fmt::Display;

pub static KROMA_MSG_HEADER: &str = "KROMA";

pub fn kroma_msg<S: AsRef<str> + Display>(msg: S) -> String {
    format!("[{KROMA_MSG_HEADER}] {msg}")
}

pub fn kroma_info<S: AsRef<str> + Display>(msg: S) {
    info!("{}", kroma_msg(msg))
}

pub fn kroma_err<S: AsRef<str> + Display>(msg: S) {
    error!("{}", kroma_msg(msg))
}
