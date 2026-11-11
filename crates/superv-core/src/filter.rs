use crate::model::NewItem;

/// Max size of a single item (4 MB, like Windows).
pub const MAX_ITEM_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
pub enum Reject {
    Empty,
    TooLarge,
}

/// Decide whether an item should be recorded.
pub fn check(item: &NewItem, max_bytes: usize) -> Result<(), Reject> {
    if item.content.is_empty() {
        return Err(Reject::Empty);
    }
    if item.content.len() > max_bytes {
        return Err(Reject::TooLarge);
    }
    Ok(())
}
