//! Persistence: slot-based game saves and typed settings.

mod settings;
mod store;

pub use settings::Settings;
pub use store::{SaveError, SaveStore};
