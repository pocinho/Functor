//! Command handlers. Each handler turns parsed arguments into core API calls and returns domain
//! values; rendering is separate (see `render`).

pub mod artifact;
pub mod init;
pub mod query;
pub mod relation;
pub mod run;
pub mod validate;
pub mod watch;
