use crate::{
    order::{OrderId, Order},
    orderbook_manager::OrderBookManager,
    price::Price,
    quantity::Qty,
    utils::BookId,
    market::MarketManager,
    level::LevelId,
};

pub struct MatchingEngine {
    pub orderbook_manager: OrderBookManager,
    pub market_manager: MarketManager,
}

impl MatchingEngine {
    pub fn new() -> Self {
        Self {
            orderbook_manager: OrderBookManager::new(),
            market_manager: MarketManager::new(),
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
        trader: Option<[u8; 20]>,
        nonce: Option<u64>,
        expiry: Option<u64>,
        signature: Option<[u8; 65]>,
    ) -> (Qty, Vec<MatchDetails>) {
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

        let mut match_details = Vec::new();

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

                    // Add match details
                    if let Some(maker_order) = self.orderbook_manager.oid_map.get(resting_order_id) {
                        match_details.push(MatchDetails {
                            maker_order: maker_order.clone(),
                            taker_order: Order::new(
                                qty,
                                LevelId(0),
                                book_id,
                                trader,
                                nonce,
                                expiry,
                                signature,
                            ),
                            exec_qty,
                            exec_price: price.absolute() as u32,
                            maker_is_buyer: !is_bid,
                        });
                    }
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
                trader,
                nonce,
                expiry,
                signature,
            );
        }

        (remaining_qty, match_details)
    }
}

#[derive(Debug)]
pub struct MatchDetails {
    pub maker_order: Order,
    pub taker_order: Order,
    pub exec_qty: Qty,
    pub exec_price: u32,
    pub maker_is_buyer: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    use rand::Rng;

    // Helper function to print match details
    fn print_match_details(
        maker_order: &Order,
        taker_id: OrderId,
        taker_trader: Option<[u8; 20]>,
        taker_nonce: Option<u64>,
        exec_qty: Qty,
        price: u32,
        is_bid: bool,
    ) {
        println!("\nMATCH DETAILS:");
        println!("---------------");
        println!("Execution Quantity: {}", exec_qty.value());
        println!("Price: {}", price);
        println!("Direction: {}", if is_bid { "BUY" } else { "SELL" });
        println!("Taker Order ID: {}", taker_id.0);

        // Maker (resting order) details
        println!("\nMAKER DETAILS:");
        if let Some(trader) = maker_order.trader() {
            println!("Address: 0x{}", hex::encode(trader));
        }
        if let Some(nonce) = maker_order.nonce() {
            println!("Nonce: {}", nonce);
        }
        if let Some(expiry) = maker_order.expiry() {
            println!("Expiry: {}", expiry);
        }

        // Taker (incoming order) details
        println!("\nTAKER DETAILS:");
        if let Some(trader) = taker_trader {
            println!("Address: 0x{}", hex::encode(trader));
        }
        if let Some(nonce) = taker_nonce {
            println!("Nonce: {}", nonce);
        }
        println!("---------------\n");
    }

    #[test]
    fn test_basic_matching() {
        let mut engine = MatchingEngine::new();

        println!("\nStarting basic matching test...");

        // Add a resting sell order
        engine.orderbook_manager.add_order(
            OrderId(1),
            BookId(0),
            Qty(100),
            100,
            false, // is_bid
            Some([1; 20]),  // Example trader address
            Some(1),        // Example nonce
            Some(u64::MAX), // Example expiry
            Some([0; 65]),  // Example signature
        );

        println!("Added resting sell order: ID(1), Qty(100), Price(100)");

        // Send in a matching buy order
        let (remaining, details) = engine.match_order(
            OrderId(2),
            BookId(0),
            Qty(60),
            100,
            true, // is_bid
            Some([2; 20]),  // Different trader
            Some(2),        // Different nonce
            Some(u64::MAX),
            Some([0; 65]),
        );

        // Get resting order details for printing
        if let Some(maker_order) = engine.orderbook_manager.oid_map.get(OrderId(1)) {
            print_match_details(
                maker_order,
                OrderId(2),
                Some([2; 20]),  // taker trader
                Some(2),        // taker nonce
                Qty(60),
                100,
                true,
            );
        }

        assert_eq!(remaining.value(), 0);
        println!("Remaining quantity: {}", remaining.value());

        // Check remaining sell order quantity
        if let Some(order) = engine.orderbook_manager.oid_map.get(OrderId(1)) {
            assert_eq!(order.qty().value(), 40);
            println!("Remaining resting order quantity: {}", order.qty().value());
        }
    }

    #[test]
    fn test_no_match_price() {
        let mut engine = MatchingEngine::new();

        println!("\nStarting no-match price test...");

        // Add a resting sell order at 100
        engine.orderbook_manager.add_order(
            OrderId(1),
            BookId(0),
            Qty(100),
            100,
            false,
            Some([1; 20]),
            Some(1),
            Some(u64::MAX),
            Some([0; 65]),
        );

        println!("Added resting sell order: ID(1), Qty(100), Price(100)");

        // Send in a buy order at 99 (shouldn't match)
        let (remaining, _) = engine.match_order(
            OrderId(2),
            BookId(0),
            Qty(60),
            99,
            true,
            Some([2; 20]),
            Some(2),
            Some(u64::MAX),
            Some([0; 65]),
        );

        println!("Attempted match with buy order: ID(2), Qty(60), Price(99)");
        println!("No match occurred due to price mismatch");
        println!("Remaining quantity: {}", remaining.value());

        assert_eq!(remaining.value(), 60);
    }

    #[test]
    fn test_multiple_matches() {
        let mut engine = MatchingEngine::new();

        // Add resting sell orders at increasing prices
        engine.orderbook_manager.add_order(
            OrderId(1), BookId(0), Qty(50), 100, false,
            Some([1; 20]), Some(1), Some(u64::MAX), Some([0; 65])
        );
        engine.orderbook_manager.add_order(
            OrderId(2), BookId(0), Qty(40), 101, false,
            Some([1; 20]), Some(1), Some(u64::MAX), Some([0; 65])
        );

        // Match with buy order that should fully execute against first two orders
        let (remaining, _) = engine.match_order(
            OrderId(4), BookId(0), Qty(90), 102, true,
            Some([2; 20]), Some(2), Some(u64::MAX), Some([0; 65])
        );

        assert_eq!(remaining.value(), 0); // Should fully match 90 against 50+40
    }

    #[test]
    fn test_matching_performance() {
        let mut engine = MatchingEngine::new();
        let num_orders = 100;
        let mut rng = rand::thread_rng();
        let mut latencies = Vec::with_capacity(num_orders);

        // Setup initial orderbook with some resting orders
        for i in 0..1000 {
            engine.orderbook_manager.add_order(
                OrderId(i as u32),
                BookId(0),
                Qty(rng.gen_range(1..=100)),
                rng.gen_range(90..110),
                rng.gen_bool(0.5),  // Random buy/sell
                Some([1; 20]),
                Some(i as u64),
                Some(u64::MAX),
                Some([0; 65]),
            );
        }

        println!("\nMATCHING ENGINE PERFORMANCE TEST");
        println!("===============================");
        println!("Processing {} orders...\n", num_orders);

        let start_time = Instant::now();
        let mut total_matches = 0;

        // Process random orders and measure latency
        for i in 1000..(1000 + num_orders) {
            let order_start = Instant::now();
            let (remaining, _) = engine.match_order(
                OrderId(i as u32),
                BookId(0),
                Qty(rng.gen_range(1..=100)),
                rng.gen_range(90..110),
                rng.gen_bool(0.5),
                Some([1; 20]),
                Some(i as u64),
                Some(u64::MAX),
                Some([0; 65]),
            );
            latencies.push(order_start.elapsed());

            if remaining.value() == 0 {
                total_matches += 1;
            }
        }

        let total_time = start_time.elapsed();
        let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
        let max_latency = latencies.iter().max().unwrap();
        let min_latency = latencies.iter().min().unwrap();
        let throughput = num_orders as f64 / total_time.as_secs_f64();

        println!("PERFORMANCE RESULTS");
        println!("-----------------");
        println!("Total Orders: {}", num_orders);
        println!("Full Matches: {}", total_matches);
        println!("Total Time: {:?}", total_time);
        println!("Throughput: {:.2} orders/sec", throughput);
        println!("\nLATENCY STATISTICS");
        println!("Average: {:?}", avg_latency);
        println!("Maximum: {:?}", max_latency);
        println!("Minimum: {:?}", min_latency);

        // Basic assertions
        assert!(throughput > 0.0);
        assert!(total_matches > 0);
    }
}