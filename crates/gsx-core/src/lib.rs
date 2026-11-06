#![forbid(unsafe_code)]

pub mod source;
pub mod syntax;

pub mod document;
pub mod serialize;
pub mod wire;

pub mod measure;
pub mod project;

pub mod forms;
pub mod targets;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
