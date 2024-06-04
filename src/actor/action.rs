use eyre::Result;

use crate::actor::outcome::Outcome;

// Excuse me clippy, that's exactly why the struct exists.
#[allow(clippy::type_complexity)]
/// An action to be run by an actor. Actions are fallible, run in the background, but cannot
/// return any values to the caller directly.
pub struct Action<A>(Box<dyn FnOnce(&mut A) -> Result<Outcome> + Send + 'static>);

impl<A> Action<A> {
    pub fn new(f: impl FnOnce(&mut A) -> Result<Outcome> + Send + 'static) -> Self {
        Self(Box::new(f))
    }

    pub fn run(self, a: &mut A) -> Result<Outcome> {
        self.0(a)
    }
}
