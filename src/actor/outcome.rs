/// The outcome of an actor action. If the action returns `Ok(Outcome::Stop)` or `Err(_)`,
/// the actor will stop. If the action returns `Ok(Outcome::Continue)`, the actor will continue.
#[derive(Debug)]
pub enum Outcome {
    Continue,
    Stop,
}
