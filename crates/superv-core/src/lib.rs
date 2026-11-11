//! Core model and persistent store for the superv clipboard history.
pub mod filter;
pub mod model;
pub mod store;

pub use model::{Content, Item, NewItem};
pub use store::{Store, StoreConfig};
