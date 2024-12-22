use std::cell::{Cell, RefCell};

thread_local! {
    pub static EXPLORATION_RATE: RefCell<f32> = RefCell::new(0.9_f32);
    pub static LEARNING_RATE: RefCell<f32> = RefCell::new(0.1_f32);
    pub static DISCOUNT_RATE: RefCell<f32> = RefCell::new(0.9_f32);
    pub static K: RefCell<f32> = RefCell::new(0.05_f32);
}

pub const NUM_EPISODES: usize = 500_000_usize;
