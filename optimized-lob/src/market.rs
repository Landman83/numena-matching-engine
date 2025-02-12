use crate::utils::BookId;

/// Configuration for a specific trading pair/market
#[derive(Debug, Clone)]
pub struct MarketConfig {
    pub base_token: [u8; 20],     // e.g. USDC
    pub security_token: [u8; 20],  // e.g. ETH
    pub fee_recipient: [u8; 20],
    pub pool: [u8; 20],
    pub signature_type: u8,
}

/// Manages market configurations for different book IDs
pub struct MarketManager {
    configs: Vec<Option<MarketConfig>>,
}

impl MarketManager {
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
        }
    }

    pub fn add_market(&mut self, book_id: BookId, config: MarketConfig) {
        let idx = book_id.value() as usize;
        if idx >= self.configs.len() {
            self.configs.resize(idx + 1, None);
        }
        self.configs[idx] = Some(config);
    }

    pub fn get_config(&self, book_id: BookId) -> Option<&MarketConfig> {
        self.configs
            .get(book_id.value() as usize)
            .and_then(|config| config.as_ref())
    }
} 