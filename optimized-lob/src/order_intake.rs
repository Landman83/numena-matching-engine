use crate::{
    order::Order,
    price::Price,
    quantity::Qty,
    utils::BookId,
};
use std::fmt;

#[derive(Debug)]
pub enum OrderIntakeError {
    InvalidQuantity,
    InvalidPrice,
    InvalidBookId,
    InvalidTrader,
    InvalidSignature,
    InvalidNonce,
}

impl fmt::Display for OrderIntakeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OrderIntakeError::InvalidQuantity => write!(f, "Invalid quantity"),
            OrderIntakeError::InvalidPrice => write!(f, "Invalid price"),
            OrderIntakeError::InvalidBookId => write!(f, "Invalid book ID"),
            OrderIntakeError::InvalidTrader => write!(f, "Invalid trader address"),
            OrderIntakeError::InvalidSignature => write!(f, "Invalid signature"),
            OrderIntakeError::InvalidNonce => write!(f, "Invalid nonce"),
        }
    }
}

/// Represents an order submission from the frontend
#[derive(Debug)]
pub struct OrderSubmission {
    pub book_id: String,
    pub price: i32,        // Changed from u64 to i32 to match Price
    pub quantity: u32,     // Changed from u64 to u32 to match Qty
    pub trader: String,
    pub nonce: u64,
    pub expiry: Option<u64>,  // Make expiry optional
    pub signature: String,
}

impl OrderSubmission {
    /// Validates and converts the submission into an internal Order
    pub fn into_order(self) -> Result<Order, OrderIntakeError> {
        // Validate quantity
        if self.quantity == 0 {
            return Err(OrderIntakeError::InvalidQuantity);
        }

        // Validate price
        if self.price == 0 {
            return Err(OrderIntakeError::InvalidPrice);
        }

        // Convert hex trader address to bytes
        let trader_bytes = hex::decode(&self.trader.trim_start_matches("0x"))
            .map_err(|_| OrderIntakeError::InvalidTrader)?;
        if trader_bytes.len() != 20 {
            return Err(OrderIntakeError::InvalidTrader);
        }
        let mut trader = [0u8; 20];
        trader.copy_from_slice(&trader_bytes);

        // Just convert signature to bytes without validation
        let sig_bytes = hex::decode(&self.signature.trim_start_matches("0x"))
            .map_err(|_| OrderIntakeError::InvalidSignature)?;
        let mut signature = [0u8; 65];
        let sig_len = std::cmp::min(sig_bytes.len(), 65);
        signature[..sig_len].copy_from_slice(&sig_bytes[..sig_len]);

        Ok(Order::new_submission(
            Qty(self.quantity),
            Price(self.price),
            BookId::from_str(&self.book_id)?,
            trader,
            self.nonce,
            self.expiry.unwrap_or(u64::MAX), // Use max value if no expiry provided
            signature,
        ))
    }
}

pub struct OrderIntake;

impl OrderIntake {
    /// Creates a new OrderIntake instance
    pub fn new() -> Self {
        Self
    }

    /// Processes an order submission and returns a validated Order
    pub fn process_submission(&self, submission: OrderSubmission) -> Result<Order, OrderIntakeError> {
        submission.into_order()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_order_submission() {
        let submission = OrderSubmission {
            book_id: "ETH-USD".to_string(),
            price: 1000,
            quantity: 100,
            trader: "0x1234567890123456789012345678901234567890".to_string(),
            nonce: 1,
            expiry: Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600),  // 1 hour from now
            signature: "0x123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345".to_string(),
        };

        let result = OrderIntake::new().process_submission(submission);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_quantity() {
        let submission = OrderSubmission {
            book_id: "ETH-USD".to_string(),
            price: 1000,
            quantity: 0,  // Invalid quantity
            trader: "0x1234567890123456789012345678901234567890".to_string(),
            nonce: 1,
            expiry: None,
            signature: "0x123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345".to_string(),
        };

        let result = OrderIntake::new().process_submission(submission);
        assert!(matches!(result, Err(OrderIntakeError::InvalidQuantity)));
    }
}
