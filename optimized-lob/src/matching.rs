use crate::{
    order::OrderId,
    orderbook_manager::OrderBookManager,
    price::Price,
    quantity::Qty,
    utils::BookId,
};

pub struct MatchingEngine {
    pub orderbook_manager: OrderBookManager,
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self {
            orderbook_manager: OrderBookManager::new(),
        }
    }

    pub fn get_orderbook_manager(&self) -> &OrderBookManager {
        &self.orderbook_manager
    }

    /// Attempts to match an incoming order against the order book
    /// Returns the remaining quantity after matching
    pub fn match_order(
        &mut self,
        order_id: OrderId,
        book_id: BookId,
        qty: Qty,
        price: u32,
        is_bid: bool,
    ) -> Qty {
        let mut remaining_qty = qty;

        // Convert price to internal format
        let price = Price::from_u32(price, is_bid);

        // Get the opposite side's best price
        let opposite_best_price = if is_bid {
            self.orderbook_manager
                .get_best_ask(book_id)
        } else {
            self.orderbook_manager
                .get_best_bid(book_id)
        };

        // Check if we can match (price crosses spread)
        let can_match = match opposite_best_price {
            Some(best_price) => {
                if is_bid {
                    price.absolute() >= best_price.absolute()
                } else {
                    price.absolute() <= best_price.absolute()
                }
            }
            None => false,
        };

        if can_match {
            // Match against resting orders until either:
            // 1. The incoming order is fully filled
            // 2. There are no more orders at acceptable prices
            while remaining_qty.value() > 0 {
                if let Some((resting_order_id, match_qty)) = self.orderbook_manager
                    .get_next_match(book_id, is_bid, price) 
                {
                    let exec_qty = std::cmp::min(remaining_qty, match_qty);
                    
                    // Execute the match
                    self.orderbook_manager.execute_order(resting_order_id, exec_qty);
                    remaining_qty -= exec_qty;
                } else {
                    break;
                }
            }
        }

        // Add any remaining quantity to the book
        if remaining_qty.value() > 0 {
            self.orderbook_manager.add_order(
                order_id,
                book_id,
                remaining_qty,
                price.absolute() as u32,
                is_bid,
            );
        }

        remaining_qty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_matching() {
        let mut engine = MatchingEngine::new();

        // Add a resting sell order
        engine.orderbook_manager.add_order(
            OrderId(1),
            BookId(0),
            Qty(100),
            100, // price
            false, // is_bid
        );

        // Send in a matching buy order
        let remaining = engine.match_order(
            OrderId(2),
            BookId(0),
            Qty(60),
            100, // price
            true, // is_bid
        );

        assert_eq!(remaining.value(), 0); // Should be fully matched

        // Check remaining sell order quantity
        if let Some(order) = engine.orderbook_manager.oid_map.get(OrderId(1)) {
            assert_eq!(order.qty().value(), 40);
        }
    }

    #[test]
    fn test_no_match_price() {
        let mut engine = MatchingEngine::new();

        // Add a resting sell order at 100
        engine.orderbook_manager.add_order(
            OrderId(1),
            BookId(0),
            Qty(100),
            100,
            false,
        );

        // Send in a buy order at 99 (shouldn't match)
        let remaining = engine.match_order(
            OrderId(2),
            BookId(0),
            Qty(60),
            99,
            true,
        );

        assert_eq!(remaining.value(), 60); // Should not match
    }
}
