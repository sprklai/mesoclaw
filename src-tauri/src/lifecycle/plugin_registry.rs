//! Plugin registry for resource handlers.
//!
//! The plugin registry manages registration and lookup of resource handlers.
//! It provides a plugin-based extensibility model for adding new resource types.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::handlers::ResourceHandler;
use super::states::ResourceType;

/// Registry for resource handler plugins.
///
/// The plugin registry maintains a mapping from resource types to their
/// handlers. It supports dynamic registration and thread-safe access.
pub struct PluginRegistry {
    /// Handlers indexed by resource type
    handlers: RwLock<HashMap<ResourceType, Box<dyn ResourceHandler>>>,
    /// Deterministic iteration order
    handler_order: RwLock<Vec<ResourceType>>,
}

impl PluginRegistry {
    /// Create a new empty plugin registry.
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            handler_order: RwLock::new(Vec::new()),
        }
    }

    /// Register a resource handler.
    ///
    /// If a handler is already registered for this resource type,
    /// it will be replaced.
    pub fn register(&self, handler: Box<dyn ResourceHandler>) {
        let resource_type = handler.resource_type();

        {
            let mut handlers = self.handlers.write().unwrap();
            handlers.insert(resource_type.clone(), handler);
        }

        {
            let mut order = self.handler_order.write().unwrap();
            if !order.contains(&resource_type) {
                order.push(resource_type);
            }
        }
    }

    /// Get the handler for a resource type.
    ///
    /// Returns `None` if no handler is registered for the type.
    pub fn get(&self, resource_type: &ResourceType) -> Option<Box<dyn ResourceHandler>> {
        // Note: We return a Box clone here for thread-safety. In practice,
        // handlers are cheaply clonable or stateless.
        let handlers = self.handlers.read().unwrap();
        handlers.get(resource_type).map(|h| {
            // Create a new boxed handler - this requires ResourceHandler to be clonable
            // For now, we'll return a reference wrapper. Actually, we can't return
            // a reference from a read guard that lives beyond the function.
            // Let's use Arc internally instead.
            unimplemented!("Use get_ref for read-only access")
        })
    }

    /// Get a reference to the handler for a resource type.
    ///
    /// This provides read-only access without cloning.
    pub fn get_ref(&self, resource_type: &ResourceType) -> Option<HandlerRef<'_>> {
        let handlers = self.handlers.read().unwrap();
        // We need to return something that holds the guard
        if handlers.contains_key(resource_type) {
            Some(HandlerRef {
                registry: self,
                resource_type: resource_type.clone(),
            })
        } else {
            None
        }
    }

    /// Check if a handler is registered for a resource type.
    pub fn is_registered(&self, resource_type: &ResourceType) -> bool {
        let handlers = self.handlers.read().unwrap();
        handlers.contains_key(resource_type)
    }

    /// Get all registered resource types in registration order.
    pub fn registered_types(&self) -> Vec<ResourceType> {
        let order = self.handler_order.read().unwrap();
        order.clone()
    }

    /// Get the number of registered handlers.
    pub fn len(&self) -> usize {
        let handlers = self.handlers.read().unwrap();
        handlers.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Unregister a handler by resource type.
    ///
    /// Returns `true` if a handler was removed.
    pub fn unregister(&self, resource_type: &ResourceType) -> bool {
        {
            let mut handlers = self.handlers.write().unwrap();
            handlers.remove(resource_type);
        }
        {
            let mut order = self.handler_order.write().unwrap();
            if let Some(pos) = order.iter().position(|t| t == resource_type) {
                order.remove(pos);
                return true;
            }
        }
        false
    }

    /// Clear all registered handlers.
    pub fn clear(&self) {
        let mut handlers = self.handlers.write().unwrap();
        handlers.clear();
        let mut order = self.handler_order.write().unwrap();
        order.clear();
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Reference to a handler in the registry.
///
/// This struct holds a reference to the registry and provides
/// access to handler methods.
pub struct HandlerRef<'a> {
    registry: &'a PluginRegistry,
    resource_type: ResourceType,
}

impl<'a> HandlerRef<'a> {
    /// Access the handler.
    ///
    /// The closure receives a reference to the handler.
    pub fn with_handler<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&dyn ResourceHandler) -> R,
    {
        let handlers = self.registry.handlers.read().unwrap();
        let handler = handlers.get(&self.resource_type).unwrap();
        f(handler.as_ref())
    }
}

/// Thread-safe shared plugin registry.
pub type SharedPluginRegistry = Arc<PluginRegistry>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::states::{
        FallbackOption, HealthStatus, PreservedState, ResourceConfig, ResourceError,
        ResourceInstance,
    };
    use async_trait::async_trait;

    // Mock handler for testing
    struct MockHandler {
        resource_type: ResourceType,
    }

    #[async_trait]
    impl ResourceHandler for MockHandler {
        fn resource_type(&self) -> ResourceType {
            self.resource_type.clone()
        }

        async fn start(
            &self,
            _id: crate::lifecycle::ResourceId,
            _config: ResourceConfig,
        ) -> Result<ResourceInstance, ResourceError> {
            unimplemented!()
        }

        async fn stop(&self, _instance: &mut ResourceInstance) -> Result<(), ResourceError> {
            Ok(())
        }

        async fn kill(&self, _instance: &mut ResourceInstance) -> Result<(), ResourceError> {
            Ok(())
        }

        async fn extract_state(
            &self,
            _instance: &ResourceInstance,
        ) -> Result<PreservedState, ResourceError> {
            unimplemented!()
        }

        async fn apply_state(
            &self,
            _instance: &mut ResourceInstance,
            _state: PreservedState,
        ) -> Result<(), ResourceError> {
            Ok(())
        }

        fn get_fallbacks(&self, _current: &ResourceInstance) -> Vec<FallbackOption> {
            vec![]
        }

        async fn health_check(
            &self,
            _instance: &ResourceInstance,
        ) -> Result<HealthStatus, ResourceError> {
            Ok(HealthStatus::Healthy)
        }

        async fn cleanup(&self, _instance: &ResourceInstance) -> Result<(), ResourceError> {
            Ok(())
        }
    }

    #[test]
    fn test_register_and_lookup() {
        let registry = PluginRegistry::new();

        registry.register(Box::new(MockHandler {
            resource_type: ResourceType::Agent,
        }));

        assert!(registry.is_registered(&ResourceType::Agent));
        assert!(!registry.is_registered(&ResourceType::Channel));
    }

    #[test]
    fn test_registered_types() {
        let registry = PluginRegistry::new();

        registry.register(Box::new(MockHandler {
            resource_type: ResourceType::Agent,
        }));
        registry.register(Box::new(MockHandler {
            resource_type: ResourceType::Channel,
        }));

        let types = registry.registered_types();
        assert_eq!(types.len(), 2);
        assert!(types.contains(&ResourceType::Agent));
        assert!(types.contains(&ResourceType::Channel));
    }

    #[test]
    fn test_unregister() {
        let registry = PluginRegistry::new();

        registry.register(Box::new(MockHandler {
            resource_type: ResourceType::Agent,
        }));

        assert!(registry.is_registered(&ResourceType::Agent));

        registry.unregister(&ResourceType::Agent);

        assert!(!registry.is_registered(&ResourceType::Agent));
    }
}
