// utils.rs

use crate::order_intake::OrderIntakeError;

pub const INITIAL_ORDER_COUNT: usize = 1 << 20;
pub const MAX_BOOKS: usize = 1 << 14;
pub const MAX_LEVELS: usize = 1 << 20;

use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct BookId(pub u32);

impl BookId {
    #[inline]
    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn from_str(s: &str) -> Result<Self, OrderIntakeError> {
        // You can implement your own logic here for converting strings to BookId
        // For example, you could use a hash function or maintain a mapping
        // This is a simple example that just hashes the string to a u32
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        Ok(BookId(hasher.finish() as u32))
    }
}
