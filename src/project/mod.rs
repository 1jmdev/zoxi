pub mod discover;
pub mod manifest;
pub mod paths;

pub use discover::discover_sources;
pub use manifest::write_generated_manifest;
pub use paths::ProjectPaths;
