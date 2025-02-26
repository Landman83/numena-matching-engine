use actix_web::{web, App, HttpResponse, HttpServer, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::order_intake::{OrderIntake, OrderSubmission, OrderIntakeError};
use crate::book_registry::{BookRegistry, BookRegistryError};

/// API request structure that matches frontend order submission format
#[derive(Deserialize, Serialize, Debug)]
pub struct OrderRequest {
    book_id: String,
    price: i32,
    quantity: u32,
    trader: String,
    nonce: u64,
    expiry: u64,
    signature: String,
}

/// API response structure
#[derive(Serialize)]
pub struct OrderResponse {
    success: bool,
    message: String,
    order_id: Option<u32>,
}

/// Shared state between handlers
pub struct AppState {
    order_intake: Arc<Mutex<OrderIntake>>,
    book_registry: Arc<BookRegistry>,
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
    match state.book_registry.register_book(data.book_id.clone()) {
        Ok(_) => Ok(HttpResponse::Ok().json(CreateBookResponse {
            success: true,
            message: "Book created successfully".to_string(),
        })),
        Err(BookRegistryError::BookAlreadyExists) => Ok(HttpResponse::BadRequest().json(CreateBookResponse {
            success: false,
            message: "Book already exists".to_string(),
        })),
        Err(_) => Ok(HttpResponse::InternalServerError().json(CreateBookResponse {
            success: false,
            message: "Failed to create book".to_string(),
        })),
    }
}

/// Add new handler for listing books
async fn list_books(state: web::Data<AppState>) -> Result<HttpResponse> {
    let books = state.book_registry.list_books();
    Ok(HttpResponse::Ok().json(ListBooksResponse { books }))
}

/// Modify the submit_order handler to verify book exists
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
        Ok(_order) => {
            // Here you would typically add the order to your orderbook
            // and get back an order ID
            let order_id = 0; // Replace with actual order ID from orderbook

            Ok(HttpResponse::Ok().json(OrderResponse {
                success: true,
                message: "Order submitted successfully".to_string(),
                order_id: Some(order_id),
            }))
        }
        Err(error) => {
            let error_message = match error {
                OrderIntakeError::InvalidQuantity => "Invalid quantity",
                OrderIntakeError::InvalidPrice => "Invalid price",
                OrderIntakeError::InvalidBookId => "Invalid book ID",
                OrderIntakeError::InvalidTrader => "Invalid trader address",
                OrderIntakeError::InvalidSignature => "Invalid signature",
                OrderIntakeError::InvalidNonce => "Invalid nonce",
                OrderIntakeError::ExpiryTooSoon => "Order expiry too soon",
                OrderIntakeError::ExpiryTooFar => "Order expiry too far",
                OrderIntakeError::InvalidExpiry => "Invalid expiry",
            };

            Ok(HttpResponse::BadRequest().json(OrderResponse {
                success: false,
                message: error_message.to_string(),
                order_id: None,
            }))
        }
    }
}

/// Configure API routes
fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/books", web::post().to(create_book))
            .route("/books", web::get().to(list_books))
            .route("/orders", web::post().to(submit_order))
    );
}

/// Start the API server
pub async fn start_server() -> std::io::Result<()> {
    let state = web::Data::new(AppState {
        order_intake: Arc::new(Mutex::new(OrderIntake::new())),
        book_registry: Arc::new(BookRegistry::new()),
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
            expiry: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600,
            signature: "0x123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345".to_string(),
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