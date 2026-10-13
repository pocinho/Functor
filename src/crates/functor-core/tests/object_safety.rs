//! Every core service trait must be usable as a trait object (ADR-0010).

use functor_core::{
    ArtifactRepository, Clock, EventPublisher, EventSource, EventSubscription, IdGenerator,
    QueryEngine, RelationshipRepository, RepositoryManager, RepositoryReader, ValidationEngine,
};

fn assert_send_sync<T: ?Sized + Send + Sync>() {}
fn assert_send<T: ?Sized + Send>() {}

/// Naming `dyn Trait` proves object safety; the bounds prove the objects can cross threads.
#[test]
fn core_traits_are_dyn_compatible_and_thread_safe() {
    assert_send_sync::<dyn ArtifactRepository>();
    assert_send_sync::<dyn RelationshipRepository>();
    assert_send_sync::<dyn RepositoryReader>();
    assert_send_sync::<dyn RepositoryManager>();
    assert_send_sync::<dyn QueryEngine>();
    assert_send_sync::<dyn ValidationEngine>();
    assert_send_sync::<dyn EventPublisher>();
    assert_send_sync::<dyn EventSource>();
    assert_send_sync::<dyn Clock>();
    assert_send_sync::<dyn IdGenerator>();
    assert_send::<dyn EventSubscription>();
}
