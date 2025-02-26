use crate::{
    order::Order,
    price::Price,
    quantity::Qty,
    utils::BookId,
};

#[derive(Debug)]
pub enum OrderIntakeError {
    InvalidQuantity,
    InvalidPrice,
    InvalidBookId,
    InvalidTrader,
    InvalidSignature,
    InvalidNonce,
    InvalidExpiry,
    ExpiryTooSoon,
    ExpiryTooFar,
}

/// Represents an order submission from the frontend
#[derive(Debug)]
pub struct OrderSubmission {
    pub book_id: String,
    pub price: i32,        // Changed from u64 to i32 to match Price
    pub quantity: u32,     // Changed from u64 to u32 to match Qty
    pub trader: String,
    pub nonce: u64,
    pub expiry: u64,
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

        // Convert hex signature to bytes
        let sig_bytes = hex::decode(&self.signature.trim_start_matches("0x"))
            .map_err(|_| OrderIntakeError::InvalidSignature)?;
        if sig_bytes.len() != 65 {
            return Err(OrderIntakeError::InvalidSignature);
        }
        let mut signature = [0u8; 65];
        signature.copy_from_slice(&sig_bytes);

        // Validate expiry
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if self.expiry <= now {
            return Err(OrderIntakeError::ExpiryTooSoon);
        }
        
        if self.expiry > now + 24 * 60 * 60 {  // 24 hours max
            return Err(OrderIntakeError::ExpiryTooFar);
        }

        Ok(Order::new_submission(
            Qty(self.quantity),    // Now u32, no conversion needed
            Price(self.price),     // Now i32, no conversion needed
            BookId::from_str(&self.book_id)?,
            trader,
            self.nonce,
            self.expiry,
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
            expiry: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600,  // 1 hour from now
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
            expiry: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600,
            signature: "0x123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345".to_string(),
        };

        let result = OrderIntake::new().process_submission(submission);
        assert!(matches!(result, Err(OrderIntakeError::InvalidQuantity)));
    }
}
