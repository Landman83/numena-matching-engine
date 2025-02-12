// order.rs

use crate::{
    level::LevelId,
    quantity::Qty,
    utils::{BookId, INITIAL_ORDER_COUNT},
};
use std::fmt::Debug;

/// Unique identifier for an order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct OrderId(pub u32);

/// Represents an order in the trading system.
#[derive(Default, Clone)]
pub struct Order {
    level_id: LevelId,
    book_id: BookId,
    qty: Qty,
    trader: Option<[u8; 20]>,      // Ethereum address as fixed bytes
    nonce: Option<u64>,            // Order nonce for signature
    expiry: Option<u64>,           // Timestamp
    signature: Option<[u8; 65]>,   // Raw signature bytes (r,s,v)
}

impl Debug for Order {
    /// Formats the Order for debugging purposes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Order")
            .field("level_id", &self.level_id)
            .field("book_id", &self.book_id)
            .field("qty", &self.qty)
            .field("trader", &self.trader)
            .field("nonce", &self.nonce)
            .field("expiry", &self.expiry)
            .field("signature", &self.signature)
            .finish()
    }
}

impl PartialEq for Order {
    fn eq(&self, other: &Self) -> bool {
        self.level_id == other.level_id && self.book_id == other.book_id && self.qty == other.qty
    }
}

impl AsRef<Order> for Order {
    fn as_ref(&self) -> &Order {
        self
    }
}

impl Order {
    /// Creates a new order with the given quantity, level ID, and book ID.
    #[inline]
    pub fn new(
        qty: Qty, 
        level_id: LevelId, 
        book_id: BookId,
        trader: Option<[u8; 20]>,
        nonce: Option<u64>,
        expiry: Option<u64>,
        signature: Option<[u8; 65]>,
    ) -> Self {
        Self {
            qty,
            level_id,
            book_id,
            trader,
            nonce,
            expiry,
            signature,
        }
    }

    /// Replaces the contents of the order with another order.
    #[inline]
    pub fn replace(&mut self, order: Order) {
        self.level_id = order.level_id;
        self.book_id = order.book_id;
        self.qty = order.qty;
    }

    /// Gets the quantity of the order.
    #[inline]
    pub fn qty(&self) -> Qty {
        self.qty
    }

    /// Gets the book ID associated with the order.
    #[inline]
    pub fn book_id(&self) -> BookId {
        self.book_id
    }

    /// Gets the level ID associated with the order.
    #[inline]
    pub fn level_id(&self) -> LevelId {
        self.level_id
    }

    /// Sets the quantity of the order.
    #[inline]
    pub fn set_qty(&mut self, qty: Qty) {
        self.qty = qty;
    }

    /// Sets the book ID of the order.
    #[inline]
    pub fn set_book_id(&mut self, book_id: BookId) {
        self.book_id = book_id;
    }

    /// Sets the level ID of the order.
    #[inline]
    pub fn set_level_id(&mut self, level_id: LevelId) {
        self.level_id = level_id;
    }

    /// Gets the trader associated with the order.
    pub fn trader(&self) -> Option<[u8; 20]> {
        self.trader
    }

    /// Gets the nonce associated with the order.
    pub fn nonce(&self) -> Option<u64> {
        self.nonce
    }

    /// Gets the expiry associated with the order.
    pub fn expiry(&self) -> Option<u64> {
        self.expiry
    }

    /// Gets the signature associated with the order.
    pub fn signature(&self) -> Option<[u8; 65]> {
        self.signature
    }
}

/// Data structure for mapping OrderIds to Order objects.
pub struct OidMap {
    data: Vec<Option<Order>>,
}

impl Default for OidMap {
    /// Creates a default OidMap instance.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl OidMap {
    /// Creates a new OidMap with an initial capacity.
    #[inline]
    pub fn new() -> Self {
        OidMap {
            data: vec![None; INITIAL_ORDER_COUNT], // Use a fixed-size array
        }
    }

    /// Reserves space for an OrderId in the map.
    #[inline]
    pub fn reserve(&mut self, oid: OrderId) {
        let idx = oid.0 as usize;
        if idx >= self.data.len() {
            self.data.resize(idx + 1, None);
        }
    }

    /// Inserts an Order into the map with a specific OrderId.
    #[inline]
    pub fn insert(&mut self, oid: OrderId, value: &Order) {
        let idx = oid.0 as usize;
        if idx >= self.data.len() {
            self.data.resize(idx + 1, None);
        }
        self.data[idx] = Some(value.clone()); // Clone only when necessary
    }

    /// Removes an Order from the map by its OrderId.
    #[inline]
    pub fn remove(&mut self, oid: OrderId) {
        let idx = oid.0 as usize;
        if idx < self.data.len() {
            self.data[idx] = None;
        }
    }

    /// Updates the quantity of an Order in the map by its OrderId.
    #[inline]
    pub fn update_qty(&mut self, oid: OrderId, qty: Qty) {
        let idx = oid.0 as usize;
        if idx < self.data.len() {
            if let Some(order) = &mut self.data[idx] {
                order.qty -= qty;
            }
        }
    }

    /// Gets a reference to an Order by its OrderId.
    #[inline]
    pub fn get(&self, oid: OrderId) -> Option<&Order> {
        let idx = oid.0 as usize;
        self.data.get(idx)?.as_ref()
    }

    /// Gets a mutable reference to an Order by its OrderId.
    #[inline]
    pub fn get_mut(&mut self, oid: OrderId) -> Option<&mut Order> {
        let idx = oid.0 as usize;
        self.data.get_mut(idx)?.as_mut()
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (OrderId, &Order)> {
        self.data
            .iter()
            .enumerate()
            .filter_map(|(i, order)| order.as_ref().map(|o| (OrderId(i as u32), o)))
    }
}
