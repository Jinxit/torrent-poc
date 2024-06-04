use crate::actor::handle::Handle;

/// Actors must implement this trait in order to receive a 'self' handle.
pub trait Actor: Sized + Send + 'static {
    /// This method is called by the actor system when the actor is started.
    fn set_handle(&mut self, _handle: &Handle<Self>) {}

    /// This method is called by the actor system when the actor is stopped.
    fn stop(&mut self) {}
}
