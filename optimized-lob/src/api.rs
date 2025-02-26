use actix_web::{web, App, HttpResponse, HttpServer, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

use crate::{
    order_intake::{OrderIntake, OrderSubmission},
    book_registry::{BookRegistry, BookRegistryError},
    orderbook::OrderBook,
    level::PriceLevel,
};

/// API request structure that matches frontend order submission format
#[derive(Deserialize, Serialize, Debug)]
pub struct OrderRequest {
    book_id: String,
    price: i32,
    quantity: u32,
    trader: String,
    nonce: u64,
    expiry: Option<u64>,
    signature: String,
}

/// API response structure
#[derive(Serialize)]
pub struct OrderResponse {
    success: bool,
    message: String,
    order_id: Option<u32>,
}

/// Response types for orderbook data
#[derive(Serialize)]
pub struct OrderbookResponse {
    bids: Vec<PriceLevelResponse>,
    asks: Vec<PriceLevelResponse>,
}

#[derive(Serialize)]
pub struct PriceLevelResponse {
    price: i32,
    size: u32,
}

/// Shared state between handlers
pub struct AppState {
    order_intake: Arc<Mutex<OrderIntake>>,
    book_registry: Arc<BookRegistry>,
    orderbooks: Arc<Mutex<HashMap<String, OrderBook>>>,
}

/// Add new request/response structures
#[derive(Deserialize)]
pub struct CreateBookRequest {
    book_id: String,
}

#[derive(Serialize)]
pub struct CreateBookResponse {
    success: bool,
    message: String,
}

#[derive(Serialize)]
pub struct ListBooksResponse {
    books: Vec<String>,
}

/// Add new handler for creating books
async fn create_book(
    data: web::Json<CreateBookRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    println!("Creating book: {}", data.book_id);
    match state.book_registry.register_book(data.book_id.clone()) {
        Ok(_) => {
            // Initialize orderbook
            let mut orderbooks = state.orderbooks.lock().await;
            orderbooks.insert(data.book_id.clone(), OrderBook::new());
            println!("Book created successfully: {}", data.book_id);
            
            Ok(HttpResponse::Ok().json(CreateBookResponse {
                success: true,
                message: "Book created successfully".to_string(),
            }))
        }
        Err(BookRegistryError::BookAlreadyExists) => {
            println!("Book already exists: {}", data.book_id);
            Ok(HttpResponse::BadRequest().json(CreateBookResponse {
                success: false,
                message: "Book already exists".to_string(),
            }))
        }
        Err(_) => {
            println!("Failed to create book: {}", data.book_id);
            Ok(HttpResponse::InternalServerError().json(CreateBookResponse {
                success: false,
                message: "Failed to create book".to_string(),
            }))
        }
    }
}

/// Add new handler for listing books
async fn list_books(state: web::Data<AppState>) -> Result<HttpResponse> {
    let books = state.book_registry.list_books();
    Ok(HttpResponse::Ok().json(ListBooksResponse { books }))
}

/// Modify the submit_order handler to skip signature verification
async fn submit_order(
    data: web::Json<OrderRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    // First verify the book exists
    if state.book_registry.get_book_id(&data.book_id).is_err() {
        return Ok(HttpResponse::BadRequest().json(OrderResponse {
            success: false,
            message: "Book does not exist".to_string(),
            order_id: None,
        }));
    }

    // Convert API request to OrderSubmission
    let submission = OrderSubmission {
        book_id: data.book_id.clone(),
        price: data.price,
        quantity: data.quantity,
        trader: data.trader.clone(),
        nonce: data.nonce,
        expiry: data.expiry,
        signature: data.signature.clone(),
    };

    // Process the order submission
    let order_intake = state.order_intake.lock().await;
    match order_intake.process_submission(submission) {
        Ok(mut order) => {
            let mut orderbooks = state.orderbooks.lock().await;
            if let Some(book) = orderbooks.get_mut(&data.book_id) {
                let price = order.price();
                let qty = order.qty();
                book.add_order(&mut order, price, qty);
                println!("Order added to book: {}", data.book_id);
            }

            Ok(HttpResponse::Ok().json(OrderResponse {
                success: true,
                message: "Order submitted successfully".to_string(),
                order_id: Some(0), // You might want to generate a real order ID
            }))
        }
        Err(error) => {
            Ok(HttpResponse::BadRequest().json(OrderResponse {
                success: false,
                message: error.to_string(),
                order_id: None,
            }))
        }
    }
}

/// Add the new endpoint handler
async fn get_orderbook(
    book_id: web::Path<String>,
    state: web::Data<AppState>,
) -> Result<HttpResponse> {
    let book_id = book_id.into_inner();
    
    // Check if book exists
    if state.book_registry.get_book_id(&book_id).is_err() {
        return Ok(HttpResponse::NotFound().json(OrderResponse {
            success: false,
            message: "Book not found".to_string(),
            order_id: None,
        }));
    }

    let orderbooks = state.orderbooks.lock().await;
    let orderbook = orderbooks.get(&book_id);

    match orderbook {
        Some(book) => {
            let bids: Vec<PriceLevelResponse> = book.bids.iter()
                .filter_map(|level| {
                    book.level_pool.get(level.level_id()).map(|l| PriceLevelResponse {
                        price: level.price().value(),
                        size: l.size().value(),
                    })
                })
                .collect();

            let asks: Vec<PriceLevelResponse> = book.asks.iter()
                .filter_map(|level| {
                    book.level_pool.get(level.level_id()).map(|l| PriceLevelResponse {
                        price: level.price().value(),
                        size: l.size().value(),
                    })
                })
                .collect();

            Ok(HttpResponse::Ok().json(OrderbookResponse { bids, asks }))
        }
        None => Ok(HttpResponse::NotFound().json(OrderResponse {
            success: false,
            message: "Orderbook not found".to_string(),
            order_id: None,
        }))
    }
}

/// Handler for canceling orders
async fn cancel_order(
    order_id: web::Path<u32>,
    _state: web::Data<AppState>,
) -> Result<HttpResponse> {
    // For now, just return a not implemented response
    Ok(HttpResponse::NotImplemented().json(OrderResponse {
        success: false,
        message: "Order cancellation not yet implemented".to_string(),
        order_id: Some(order_id.into_inner()),
    }))
}

/// Configure API routes
fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/books", web::post().to(create_book))
            .route("/books", web::get().to(list_books))
            .route("/orders", web::post().to(submit_order))
            .route("/books/{book_id}/orderbook", web::get().to(get_orderbook))
            .route("/orders/{order_id}", web::delete().to(cancel_order))
    );
}

/// Start the API server
pub async fn start_server() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        order_intake: Arc::new(Mutex::new(OrderIntake::new())),
        book_registry: Arc::new(BookRegistry::new()),
        orderbooks: Arc::new(Mutex::new(HashMap::new())),
    });

    println!("Starting API server on 127.0.0.1:8080");

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(actix_web::middleware::Logger::default())
            .configure(configure_app)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};

    #[actix_web::test]
    async fn test_submit_order() {
        // Create test app
        let state = web::Data::new(AppState {
            order_intake: Arc::new(Mutex::new(OrderIntake::new())),
            book_registry: Arc::new(BookRegistry::new()),
            orderbooks: Arc::new(Mutex::new(HashMap::new())),
        });

        let app = test::init_service(
            App::new()
                .app_data(state.clone())
                .configure(configure_app)
        ).await;

        // Create test order
        let order = OrderRequest {
            book_id: "ETH-USD".to_string(),
            price: 1000,
            quantity: 100,
            trader: "0x1234567890123456789012345678901234567890".to_string(),
            nonce: 1,
            expiry: None,
            signature: String::new(),
        };

        // Send test request
        let req = test::TestRequest::post()
            .uri("/api/orders")
            .set_json(&order)
            .to_request();

        let resp: OrderResponse = test::call_and_read_body_json(&app, req).await;
        assert!(resp.success);
    }
} 