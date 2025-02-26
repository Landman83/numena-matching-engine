// pool.rs

// Import the Level and LevelId structs from the level module.
use crate::level::{Level, LevelId};
use crate::utils::MAX_LEVELS;

// Define a struct named LevelPool, which is a pool for managing Level objects.
#[derive(Clone)]
pub struct LevelPool {
    levels: Vec<Level>, // A vector to store allocated Level objects.
    free_list: Vec<LevelId>,    // A vector to store free LevelId values.
}

impl LevelPool {
    // Constructor for creating a new LevelPool instance with default values.
    #[inline]
    pub fn new() -> Self {
        Self {
            levels: Vec::new(), // Initialize allocated vector as empty.
            free_list: Vec::new(),      // Initialize free vector as empty.
        }
    }

    // Constructor for creating a new LevelPool instance with a specified capacity.
    pub fn new_with_capacity(capacity: usize) -> Self {
        Self {
            levels: Vec::with_capacity(capacity), // Initialize allocated vector with the specified capacity.
            free_list: Vec::with_capacity(capacity), // Initialize free vector with the specified capacity.
        }
    }

    // Allocate a LevelId from the pool. Reuses a free LevelId if available or creates a new one.
    pub fn alloc(&mut self) -> LevelId {
        if let Some(id) = self.free_list.pop() {
            id
        } else {
            let id = LevelId(self.levels.len() as u32);
            self.levels.push(Level::default());
            id
        }
    }

    // Free a LevelId by adding it back to the pool of available LevelIds.
    pub fn free(&mut self, id: LevelId) {
        self.free_list.push(id);
    }

    // Get a reference to a Level by LevelId if it exists in the pool.
    #[inline]
    pub fn get(&self, id: LevelId) -> Option<&Level> {
        self.levels.get(id.0 as usize)
    }

    // Get a mutable reference to a Level by LevelId if it exists in the pool.
    pub fn get_mut(&mut self, id: LevelId) -> Option<&mut Level> {
        self.levels.get_mut(id.0 as usize)
    }

    // Set the Level object associated with a LevelId in the pool.
    pub fn set_level(&mut self, id: LevelId, level: Level) {
        if let Some(existing_level) = self.levels.get_mut(id.0 as usize) {
            *existing_level = level;
        }
    }
}
