use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, TryLockError};
use std::thread::JoinHandle;

use eyre::{eyre, Result};
use tracing::error;

use crate::actor::action::Action;
use crate::actor::actor::Actor;
use crate::actor::outcome::Outcome;

/// A handle to an actor. It can be used to send actions to the actor, and to stop it.
#[derive(Debug)]
pub struct Handle<A>
where
    A: Actor + Send + 'static,
{
    join_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    sender: Sender<Action<A>>,
}

// Manual Clone implementation because A does not need to be Clone for Handle<A> to be Clone.
impl<A> Clone for Handle<A>
where
    A: Actor + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            join_handle: self.join_handle.clone(),
            sender: self.sender.clone(),
        }
    }
}

impl<A> Handle<A>
where
    A: Actor + Send + 'static,
{
    /// Turns almost any Send self-mutating type into an actor.
    /// The only requirement is that it implements the Actor trait.
    pub fn spawn(mut actor: A) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel::<Action<A>>();
        let join_handle = Arc::new(Mutex::new(None));
        let s = Self {
            join_handle: join_handle.clone(),
            sender,
        };
        actor.set_handle(&s);
        *join_handle.lock().expect("mutex to not be poisoned") =
            Some(std::thread::spawn(move || {
                while let Ok(action) = receiver.recv() {
                    let outcome = action.run(&mut actor);
                    match outcome {
                        Ok(Outcome::Continue) => {}
                        Ok(Outcome::Stop) => break,
                        Err(e) => {
                            error!("Unhandled error in actor thread: {:?}", e);
                            break;
                        }
                    }
                }
                actor.stop();
            }));
        s
    }

    /// Enqueue an action to be run by the actor thread.
    /// The action will not be able to return any values, and will be run in the background.
    pub fn act(&self, f: impl FnOnce(&mut A) -> Result<Outcome> + Send + 'static) -> Result<()> {
        self.sender
            .send(Action::new(f))
            .map_err(|_| eyre!("Failed to send action to actor"))
    }

    /// Stop the actor thread. This will give the actor thread a chance to finish its currently
    /// queued actions, and then stop itself.
    /// This will block until the actor thread has stopped, or return immediately if it is already
    /// stopped or is currently being stopped by another thread.
    pub fn stop(&self) {
        // Attempt to stop the actor thread if it isn't already stopped.
        // TODO: Use a separate high-priority one-shot channel to signal the actor thread to stop.
        let _ = self.act(|_| Ok(Outcome::Stop));
        match self.join_handle.try_lock() {
            Ok(mut guard) => {
                if let Some(handle) = guard.take() {
                    if let Err(e) = handle.join() {
                        error!("Failed to join actor thread: {:?}", e);
                    }
                }
            }
            Err(TryLockError::WouldBlock) => {
                // This is fine, we can get into circular dependencies with handles being cloned around.
                // If this would block, the actor thread is already about to be stopped from another thread.
            }
            Err(TryLockError::Poisoned(_)) => {
                panic!("Actor thread poisoned");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::actor::actor::Actor;
    use crate::actor::handle::Handle;

    #[derive(Debug, Default, Clone)]
    struct TestActor {
        handle: Arc<Mutex<Option<Handle<TestActor>>>>,
    }

    impl Actor for TestActor {
        fn set_handle(&mut self, handle: &Handle<TestActor>) {
            *self.handle.lock().unwrap() = Some(handle.clone());
        }
    }

    #[test]
    fn handle_is_set_after_spawn() {
        let actor = TestActor::default();
        Handle::spawn(actor.clone());
        assert!(actor.handle.lock().unwrap().is_some());
    }
}
