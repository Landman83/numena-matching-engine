use std::collections::HashMap;
use std::sync::RwLock;
use crate::utils::BookId;

#[derive(Debug)]
pub enum BookRegistryError {
    BookAlreadyExists,
    BookNotFound,
    InvalidBookId,
}

pub struct BookRegistry {
    books: RwLock<HashMap<String, BookId>>,
}

impl BookRegistry {
    pub fn new() -> Self {
        Self {
            books: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_book(&self, book_name: String) -> Result<BookId, BookRegistryError> {
        let mut books = self.books.write().unwrap();
        if books.contains_key(&book_name) {
            return Err(BookRegistryError::BookAlreadyExists);
        }

        // Create a new BookId using a deterministic method
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        book_name.hash(&mut hasher);
        let book_id = BookId(hasher.finish() as u32);

        books.insert(book_name, book_id);
        Ok(book_id)
    }

    pub fn get_book_id(&self, book_name: &str) -> Result<BookId, BookRegistryError> {
        let books = self.books.read().unwrap();
        books.get(book_name)
            .copied()
            .ok_or(BookRegistryError::BookNotFound)
    }

    pub fn list_books(&self) -> Vec<String> {
        let books = self.books.read().unwrap();
        books.keys().cloned().collect()
    }
} 