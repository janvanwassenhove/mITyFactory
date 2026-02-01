//! Station registry for managing station implementations.

use std::collections::HashMap;
use std::sync::Arc;

use tracing::debug;

use crate::error::{CoreError, CoreResult};
use crate::station::Station;

/// A registry of station implementations.
///
/// The registry maps station names to their implementations,
/// allowing dynamic lookup and execution of stations.
#[derive(Default)]
pub struct StationRegistry {
    stations: HashMap<String, Arc<dyn Station>>,
}

impl StationRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            stations: HashMap::new(),
        }
    }

    /// Register a station implementation.
    ///
    /// The station is registered under its `name()` identifier.
    /// If a station with the same name already exists, it will be replaced.
    pub fn register(&mut self, station: Arc<dyn Station>) {
        let name = station.name().to_string();
        debug!("Registering station: {}", name);
        self.stations.insert(name, station);
    }

    /// Register a station under a custom name.
    pub fn register_as(&mut self, name: impl Into<String>, station: Arc<dyn Station>) {
        let name = name.into();
        debug!("Registering station as: {}", name);
        self.stations.insert(name, station);
    }

    /// Get a station by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Station>> {
        self.stations.get(name).cloned()
    }

    /// Get a station by name, returning an error if not found.
    pub fn get_required(&self, name: &str) -> CoreResult<Arc<dyn Station>> {
        self.get(name)
            .ok_or_else(|| CoreError::StationNotFound(name.to_string()))
    }

    /// Check if a station is registered.
    pub fn contains(&self, name: &str) -> bool {
        self.stations.contains_key(name)
    }

    /// Get all registered station names.
    pub fn names(&self) -> Vec<&str> {
        self.stations.keys().map(|s| s.as_str()).collect()
    }

    /// Get the number of registered stations.
    pub fn len(&self) -> usize {
        self.stations.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.stations.is_empty()
    }

    /// Remove a station from the registry.
    pub fn unregister(&mut self, name: &str) -> Option<Arc<dyn Station>> {
        debug!("Unregistering station: {}", name);
        self.stations.remove(name)
    }

    /// Clear all registered stations.
    pub fn clear(&mut self) {
        debug!("Clearing station registry");
        self.stations.clear();
    }
}

impl std::fmt::Debug for StationRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StationRegistry")
            .field("stations", &self.stations.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::WorkflowContext;
    use crate::station::{StationInput, StationOutput, StationResult};
    use async_trait::async_trait;
    use std::path::PathBuf;

    struct TestStation {
        name: String,
    }

    #[async_trait]
    impl Station for TestStation {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "Test station"
        }

        fn input(&self) -> StationInput {
            StationInput::default()
        }

        fn output(&self) -> StationOutput {
            StationOutput::default()
        }

        async fn execute(&self, _context: &mut WorkflowContext) -> CoreResult<StationResult> {
            Ok(StationResult::success(&self.name))
        }
    }

    #[test]
    fn test_registry_register() {
        let mut registry = StationRegistry::new();
        assert!(registry.is_empty());

        registry.register(Arc::new(TestStation {
            name: "test-station".to_string(),
        }));

        assert_eq!(registry.len(), 1);
        assert!(registry.contains("test-station"));
    }

    #[test]
    fn test_registry_get() {
        let mut registry = StationRegistry::new();
        registry.register(Arc::new(TestStation {
            name: "scaffold".to_string(),
        }));

        let station = registry.get("scaffold");
        assert!(station.is_some());
        assert_eq!(station.unwrap().name(), "scaffold");

        let missing = registry.get("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_registry_names() {
        let mut registry = StationRegistry::new();
        registry.register(Arc::new(TestStation {
            name: "scaffold".to_string(),
        }));
        registry.register(Arc::new(TestStation {
            name: "validate".to_string(),
        }));

        let mut names = registry.names();
        names.sort();
        assert_eq!(names, vec!["scaffold", "validate"]);
    }

    #[test]
    fn test_registry_unregister() {
        let mut registry = StationRegistry::new();
        registry.register(Arc::new(TestStation {
            name: "test".to_string(),
        }));

        assert!(registry.contains("test"));
        registry.unregister("test");
        assert!(!registry.contains("test"));
    }
}
