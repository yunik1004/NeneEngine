//! Persistence: slot-based game saves and typed settings.

mod store;
mod settings;

pub use store::{SaveError, SaveStore};
pub use settings::Settings;
