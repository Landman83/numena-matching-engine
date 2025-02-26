mod api;
mod book_registry;
mod level;
mod order;
mod order_intake;
mod utils;
mod orderbook_manager;
mod market;
mod price;
mod quantity;
mod matching;
mod orderbook;
mod pool;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize your orderbook and other components here
    
    // Start the API server
    api::start_server().await
} 