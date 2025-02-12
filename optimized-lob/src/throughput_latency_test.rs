use crate::{
    matching::MatchingEngine,
    order::OrderId,
    quantity::Qty,
    utils::BookId,
    market::MarketConfig,
    translator::translate_matches,
};
use rand::Rng;
use std::time::{Duration, Instant};
use hex;

#[derive(Debug)]
pub struct TestOrder {
    pub order_id: OrderId,
    pub price: u32,
    pub quantity: u32,
    pub is_bid: bool,
}

#[derive(Debug)]
pub struct TestStats {
    pub total_orders: usize,
    pub total_matches: usize,
    pub total_time: Duration,
    pub avg_latency: Duration,
    pub throughput: f64,
}

pub fn run_matching_test(order_count: usize) {
    let start_time = Instant::now();
    let mut total_matches = 0;
    let mut latencies = Vec::new();

    // 1. Setup
    let mut engine = MatchingEngine::new();
    
    // Setup market config
    let market_config = MarketConfig {
        base_token: [1; 20],      // Example USDC
        security_token: [2; 20],   // Example ETH
        fee_recipient: [3; 20],
        pool: [4; 20],
        signature_type: 1,
    };
    engine.market_manager.add_market(BookId(0), market_config);

    println!("\nORDER MATCHING TEST");
    println!("===================");
    println!("Generating and processing {} orders...\n", order_count);
    
    let mut rng = rand::thread_rng();
    
    for i in 0..order_count {
        let order = TestOrder {
            order_id: OrderId(i as u32),
            price: rng.gen_range(90..=110),
            quantity: rng.gen_range(1..=100),
            is_bid: rng.gen_bool(0.5),
        };

        println!("Order {}", order.order_id.0);
        println!("---------------");
        println!("Type: {}", if order.is_bid { "BUY" } else { "SELL" });
        println!("Quantity: {}", order.quantity);
        println!("Price: {}", order.price);

        let order_start = Instant::now();
        let (_, matches) = engine.match_order(
            order.order_id,
            BookId(0),
            Qty(order.quantity),
            order.price,
            order.is_bid,
            Some([rng.gen::<u8>(); 20]),
            Some(i as u64),
            Some(u64::MAX),
            Some([0; 65]),
        );
        latencies.push(order_start.elapsed());

        if !matches.is_empty() {
            total_matches += matches.len();
            println!("\nMATCHES FOUND: {}", matches.len());
            
            let market_config = engine.market_manager.get_config(BookId(0)).unwrap();
            let settlements = translate_matches(matches, market_config);
            
            for settlement in settlements {
                println!("\nSETTLEMENT DETAILS");
                println!("------------------");
                println!("Maker: 0x{}", hex::encode(settlement.maker));
                println!("Taker: 0x{}", hex::encode(settlement.taker));
                println!("Maker Token: {} {}", 
                    settlement.maker_amount,
                    if settlement.maker_is_buyer { "USDC" } else { "ETH" });
                println!("Taker Token: {} {}", 
                    settlement.taker_amount,
                    if settlement.maker_is_buyer { "ETH" } else { "USDC" });
                println!("Price: {} USDC/ETH", 
                    if settlement.maker_is_buyer {
                        settlement.maker_amount / settlement.taker_amount
                    } else {
                        settlement.taker_amount / settlement.maker_amount
                    });
            }
        } else {
            println!("No matches found");
        }
        println!("");
    }

    // Calculate and print performance stats
    let total_time = start_time.elapsed();
    let avg_latency = latencies.iter().sum::<Duration>() / latencies.len() as u32;
    let throughput = (total_matches + order_count) as f64 / total_time.as_secs_f64();

    println!("\nPERFORMANCE STATISTICS");
    println!("=====================");
    println!("Total Orders Processed: {}", total_matches + order_count);
    println!("New Orders: {}", order_count);
    println!("Matches: {}", total_matches);
    println!("Total Time: {:?}", total_time);
    println!("Average Latency: {:?}", avg_latency);
    println!("Throughput: {:.2} orders/second", throughput);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matching_and_settlement() {
        run_matching_test(1000);
    }
} 