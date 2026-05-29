pub mod mock;
pub mod ui;

pub use mock::*;
pub use ui::*;

use hermes::utilities::NvimRuntime;

/// Creates a smol LocalExecutor for testing
pub fn mock_runtime() -> NvimRuntime {
    NvimRuntime::new()
}
