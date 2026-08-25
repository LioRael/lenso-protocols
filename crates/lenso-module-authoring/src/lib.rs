//! Runtime-neutral authoring primitives for strongly typed Lenso Modules.

use std::{cell::OnceCell, ops::Deref, rc::Rc};

/// One Module operation failure with an explicit Domain/Runtime split.
///
/// Ordinary operations can return `Result<T, DomainError>` directly. Use this
/// type only when Module code must deliberately surface an Adapter-specific
/// runtime failure in addition to its Capability-defined Domain Errors.
#[derive(Clone, Debug, PartialEq)]
pub enum ModuleError<DomainError, RuntimeError> {
    /// An expected Capability-defined business rejection.
    Domain(DomainError),
    /// An infrastructure or execution failure outside the Capability contract.
    Runtime(RuntimeError),
}

impl<DomainError, RuntimeError> ModuleError<DomainError, RuntimeError> {
    /// Creates a Capability-defined Domain Error.
    pub const fn domain(error: DomainError) -> Self {
        Self::Domain(error)
    }

    /// Creates an Adapter-specific Runtime Error.
    pub const fn runtime(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }

    /// Maps the Domain Error while preserving the Runtime Error.
    pub fn map_domain<Other>(
        self,
        map: impl FnOnce(DomainError) -> Other,
    ) -> ModuleError<Other, RuntimeError> {
        match self {
            Self::Domain(error) => ModuleError::Domain(map(error)),
            Self::Runtime(error) => ModuleError::Runtime(error),
        }
    }
}

/// A generated, strongly typed client for one required Capability.
///
/// Capability binding generators implement this trait for their client type so
/// Module authoring frontends can connect typed Ports without knowing the
/// Capability's operation kinds or handle layout. Implementations must use only
/// the supplied Plan-owned dependencies; they must not perform discovery.
pub trait CapabilityClient: Sized + 'static {
    /// Adapter-owned dependency view used to connect this client.
    type Dependencies: ?Sized;
    /// Adapter-owned failure returned when connection cannot complete.
    type Error;

    /// Stable Capability identity required by this client.
    const CAPABILITY_ID: &'static str;
    /// Exact Descriptor version understood by this generated client.
    const DESCRIPTOR_VERSION: &'static str;

    /// Connects this client to one Module Instance's resolved dependencies.
    fn from_dependencies(dependencies: &Self::Dependencies) -> Result<Self, Self::Error>;

    /// Creates the adapter failure for an invalid second connection attempt.
    fn already_connected() -> Self::Error;
}

/// A typed, lifecycle-bound Capability requirement declared by a Module.
///
/// Generated Module glue connects the Port during activation. Module behavior
/// can then call the generated Capability client directly through `Deref`.
/// A fresh Module generation owns fresh Ports; reconnecting one Port is an
/// invalid lifecycle transition.
pub struct Port<C: CapabilityClient> {
    client: Rc<OnceCell<C>>,
}

impl<C: CapabilityClient> Port<C> {
    /// Creates a disconnected typed Port.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: Rc::new(OnceCell::new()),
        }
    }

    /// Connects the Port from this Module Instance's resolved dependencies.
    pub fn connect(&self, dependencies: &C::Dependencies) -> Result<(), C::Error> {
        let client = C::from_dependencies(dependencies)?;
        self.client.set(client).map_err(|_| C::already_connected())
    }

    /// Returns whether lifecycle activation connected this Port.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.client.get().is_some()
    }
}

impl<C: CapabilityClient> Clone for Port<C> {
    fn clone(&self) -> Self {
        Self {
            client: Rc::clone(&self.client),
        }
    }
}

impl<C: CapabilityClient> std::fmt::Debug for Port<C> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Port")
            .field("capability_id", &C::CAPABILITY_ID)
            .field("descriptor_version", &C::DESCRIPTOR_VERSION)
            .field("connected", &self.is_connected())
            .finish()
    }
}

impl<C: CapabilityClient> Default for Port<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: CapabilityClient> Deref for Port<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        self.client.get().unwrap_or_else(|| {
            panic!(
                "Capability Port {} was used before Module activation",
                C::CAPABILITY_ID
            )
        })
    }
}

/// Common imports for a Module authoring frontend.
pub mod prelude {
    pub use crate::{CapabilityClient, ModuleError, Port};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Eq, PartialEq)]
    struct ExampleClient(u64);

    #[derive(Debug, Eq, PartialEq)]
    enum ExampleError {
        AlreadyConnected,
    }

    impl CapabilityClient for ExampleClient {
        type Dependencies = ();
        type Error = ExampleError;

        const CAPABILITY_ID: &'static str = "example.echo@1";
        const DESCRIPTOR_VERSION: &'static str = "1.0.0";

        fn from_dependencies(_dependencies: &Self::Dependencies) -> Result<Self, Self::Error> {
            Ok(Self(42))
        }

        fn already_connected() -> Self::Error {
            ExampleError::AlreadyConnected
        }
    }

    #[test]
    fn port_connects_once_and_is_shared_by_module_clones() {
        let port = Port::<ExampleClient>::new();
        let module_clone = port.clone();
        assert!(!port.is_connected());

        port.connect(&())
            .expect("the generated client should connect");

        assert!(module_clone.is_connected());
        assert_eq!(module_clone.0, 42);
        assert_eq!(port.connect(&()), Err(ExampleError::AlreadyConnected));
    }

    #[test]
    fn module_error_preserves_runtime_failures_while_mapping_domain_errors() {
        let domain = ModuleError::<_, &str>::domain("missing").map_domain(str::len);
        assert_eq!(domain, ModuleError::Domain(7));

        let runtime = ModuleError::<&str, _>::runtime("cancelled").map_domain(str::len);
        assert_eq!(runtime, ModuleError::Runtime("cancelled"));
    }
}
