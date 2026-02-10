//! Test utilities module.

use lazy_static::lazy_static;
use std::sync::Mutex;

pub mod mock_objects;
mod test_utils;

lazy_static! {
    /// Synchronizes tests that mutate the process working directory.
    pub static ref CWD_MUTEX: Mutex<()> = Mutex::new(());
}
