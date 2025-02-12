// translates match results into settlement format

use crate::{
    order::Order,
    quantity::Qty,
    market::MarketConfig,
    matching::MatchDetails,
};

/// Represents a signature for settlement
#[derive(Debug, Clone)]
pub struct SettlementSignature {
    pub signature_type: u8,
    pub v: u8,
    pub r: [u8; 32],
    pub s: [u8; 32],
}

/// Represents an order ready for settlement
#[derive(Debug, Clone)]
pub struct SettlementOrder {
    pub maker_token: [u8; 20],      // Address of token maker is selling/buying
    pub taker_token: [u8; 20],      // Address of token taker is selling/buying
    pub maker_amount: u128,         // Amount of maker_token
    pub taker_amount: u128,         // Amount of taker_token
    pub maker: [u8; 20],           // Maker's address
    pub taker: [u8; 20],           // Taker's address
    pub fee_recipient: [u8; 20],    // Address receiving fees
    pub pool: [u8; 20],            // Liquidity pool address if applicable
    pub expiration: u64,           // Order expiration timestamp
    pub salt: u128,                // Unique order identifier
    pub maker_is_buyer: bool,      // True if maker is buying taker_token
    pub maker_signature: SettlementSignature,
    pub taker_signature: SettlementSignature,
}

/// Translates a matched order pair into settlement format
#[allow(clippy::cast_possible_truncation)]  // Allow u32 to u128 casts
pub fn translate_to_settlement(
    maker_order: &Order,
    taker_order: &Order,
    exec_qty: Qty,
    exec_price: u32,
    maker_is_buyer: bool,
    market_config: &MarketConfig,
) -> Option<SettlementOrder> {
    // Extract signatures if available with market's signature type
    let maker_signature = maker_order.signature()
        .map(|sig| extract_signature(sig, market_config.signature_type))?;
    let taker_signature = taker_order.signature()
        .map(|sig| extract_signature(sig, market_config.signature_type))?;

    // Determine maker/taker tokens based on who is buying
    let (maker_token, taker_token) = if maker_is_buyer {
        (market_config.base_token, market_config.security_token)
    } else {
        (market_config.security_token, market_config.base_token)
    };

    // Calculate amounts based on executed quantity and price
    let (maker_amount, taker_amount) = if maker_is_buyer {
        (u128::from(exec_price) * u128::from(exec_qty.value()), 
         u128::from(exec_qty.value()))
    } else {
        (u128::from(exec_qty.value()), 
         u128::from(exec_price) * u128::from(exec_qty.value()))
    };

    // Get trader addresses
    let maker = maker_order.trader()?;
    let taker = taker_order.trader()?;

    // Get expiration (use maker's expiry)
    let expiration = maker_order.expiry()?;

    // Get salt from maker's nonce
    let salt = u128::from(maker_order.nonce()?);

    Some(SettlementOrder {
        maker_token,
        taker_token,
        maker_amount,
        taker_amount,
        maker,
        taker,
        fee_recipient: market_config.fee_recipient,
        pool: market_config.pool,
        expiration,
        salt,
        maker_is_buyer,
        maker_signature,
        taker_signature,
    })
}

/// Extracts signature components from raw bytes
fn extract_signature(sig: [u8; 65], sig_type: u8) -> SettlementSignature {
    SettlementSignature {
        signature_type: sig_type,  // Use the market config's signature type
        v: sig[64],
        r: sig[..32].try_into().unwrap(),
        s: sig[32..64].try_into().unwrap(),
    }
}

/// Translates a batch of matches into settlement orders
pub fn translate_matches(
    matches: Vec<MatchDetails>,
    market_config: &MarketConfig,
) -> Vec<SettlementOrder> {
    matches
        .into_iter()
        .filter_map(|match_details| {
            translate_to_settlement(
                &match_details.maker_order,
                &match_details.taker_order,
                match_details.exec_qty,
                match_details.exec_price,
                match_details.maker_is_buyer,
                market_config,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        matching::MatchingEngine,
        order::OrderId,
        utils::BookId,
    };

    #[test]
    fn test_full_match_and_translate_flow() {
        // Setup matching engine with market config
        let mut engine = MatchingEngine::new();
        
        // Create and add market config for BookId(0)
        let market_config = MarketConfig {
            base_token: [1; 20],      // Example USDC address
            security_token: [2; 20],   // Example ETH address
            fee_recipient: [3; 20],
            pool: [4; 20],
            signature_type: 1,
        };
        engine.market_manager.add_market(BookId(0), market_config.clone());

        // Add a resting sell order
        engine.orderbook_manager.add_order(
            OrderId(1), 
            BookId(0), 
            Qty(50), 
            100,  // price
            false, // is sell
            Some([5; 20]),  // maker address
            Some(1),        // nonce
            Some(u64::MAX), // expiry
            Some([1; 65]),  // signature
        );

        // Execute matching buy order
        let (remaining, matches) = engine.match_order(
            OrderId(3),
            BookId(0),
            Qty(30),
            100,  // price
            true, // is buy
            Some([7; 20]),  // taker address
            Some(3),        // nonce
            Some(u64::MAX), // expiry
            Some([3; 65]),  // signature
        );

        // Get market config and translate matches
        let market_config = engine.market_manager.get_config(BookId(0))
            .expect("Market config should exist");
        let settlements = translate_matches(matches, market_config);

        // Print and verify settlements
        println!("\nSETTLEMENT DETAILS:");
        for (i, settlement) in settlements.iter().enumerate() {
            println!("Settlement {}:", i + 1);
            println!("---------------");
            println!("Maker: 0x{}", hex::encode(settlement.maker));
            println!("Taker: 0x{}", hex::encode(settlement.taker));
            println!("Maker Token: 0x{}", hex::encode(settlement.maker_token));
            println!("Taker Token: 0x{}", hex::encode(settlement.taker_token));
            println!("Fee Recipient: 0x{}", hex::encode(settlement.fee_recipient));
            println!("Pool: 0x{}", hex::encode(settlement.pool));
            println!("Maker Amount: {}", settlement.maker_amount);
            println!("Taker Amount: {}", settlement.taker_amount);
            println!("Maker is Buyer: {}", settlement.maker_is_buyer);
            println!("\nMaker Signature:");
            println!("  Type: {}", settlement.maker_signature.signature_type);
            println!("  v: {}", settlement.maker_signature.v);
            println!("  r: 0x{}", hex::encode(settlement.maker_signature.r));
            println!("  s: 0x{}", hex::encode(settlement.maker_signature.s));
            println!("\nTaker Signature:");
            println!("  Type: {}", settlement.taker_signature.signature_type);
            println!("  v: {}", settlement.taker_signature.v);
            println!("  r: 0x{}", hex::encode(settlement.taker_signature.r));
            println!("  s: 0x{}", hex::encode(settlement.taker_signature.s));
            println!("---------------\n");
        }

        // Verify results
        assert_eq!(remaining.value(), 0);
        assert_eq!(settlements.len(), 1);
        
        let settlement = &settlements[0];
        assert_eq!(settlement.maker_amount, 30); // Security amount
        assert_eq!(settlement.taker_amount, 3000); // Base amount (30 * 100)
        assert_eq!(settlement.maker_is_buyer, false);
        assert_eq!(settlement.maker_signature.signature_type, 1);
        assert_eq!(settlement.fee_recipient, [3; 20]);
        assert_eq!(settlement.pool, [4; 20]);
        assert_eq!(settlement.maker_signature.v, 1);  // From [1; 65] signature
        assert_eq!(settlement.maker_signature.r, [1; 32]);
        assert_eq!(settlement.maker_signature.s, [1; 32]);
        assert_eq!(settlement.taker_signature.v, 3);  // From [3; 65] signature
        assert_eq!(settlement.taker_signature.r, [3; 32]);
        assert_eq!(settlement.taker_signature.s, [3; 32]);
    }
}

