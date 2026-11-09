//! Changes made from the local app: pure functions over a [`Project`](crate::model::Project).

mod ops;
mod tree;

pub use ops::{NewTask, Summary, TaskPatch, comment, create, delete, update};
pub use tree::{find, next_id};
