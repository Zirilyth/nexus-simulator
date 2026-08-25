pub(crate) mod systems;
pub(crate) mod types;
use crate::systems::commands::apply_commands;
use crate::systems::power::process;
pub use crate::types::world::{Connection, LoadError, World};
pub use types::catalogue::Part;
pub use types::condition::Condition;
pub use types::events::{Command, Event, EventId, EventKind, RejectReason};
pub use types::ids::NetworkKind;
pub use types::modules::{ModuleId, ModuleMeta, PortName};
pub use types::role::PowerRole;
pub use types::symptom::Symptom;
#[must_use = "events are history. drop them deliberately or not at all"]
pub fn tick(world: &mut World, commands: &[Command]) -> Vec<Event> {
	let first_new = world.log.len();

	// trips scheduled last tick open first: a thermal breaker acts late, and
	// the delay is what keeps a trip from feeding back into its own cause.
	process(world);
	apply_commands(world, commands);

	world.tick += 1;
	world.log[first_new..].to_vec()
}

/// Fold a command script over a fresh world — `U(seed, log)`, executable.
/// Savegames, bug reports, and the history engine are all this one trick.
///
/// # Errors
/// - [`LoadError::Parse`] — malformed RON (span included)
/// - [`LoadError::UnknownLabel`] — a connection references a module not aboard
pub fn replay(fixture_text: &str, script: &[Vec<Command>]) -> Result<World, LoadError> {
	let mut world = World::from_fixture_str(fixture_text)?;
	for batch in script {
		let _ = tick(&mut world, batch);
	}
	Ok(world)
}

pub fn advance_to(world: &mut World, t: u64) -> Vec<Event> {
	let first_new = world.log.len();

	while let Some(next) = world.next_event_at() {
		if next > t {
			break;
		}
		world.tick = next.max(world.tick);
		process(world);
		//
	}
	world.tick = t;
	world.log[first_new..].to_vec()
}

#[cfg(test)]
mod tests {
	use super::*;
	use rand_chacha::rand_core::RngCore;

	/// Whole-history determinism belongs to the canaries. This owns the one
	/// link they cannot see: that the seed in the fixture is the seed the
	/// universe runs on, and that a different seed is a different universe.
	#[test]
	fn the_fixture_seeds_the_universe() {
		let text = std::fs::read_to_string(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/../../fixtures/testudo.ron"
		))
		.expect("testudo should be readable");

		let mut a = World::from_fixture_str(&text).expect("testudo should load");
		let mut b = World::from_fixture_str(&text).expect("testudo should load");
		assert_eq!(
			a.rng.next_u64(),
			b.rng.next_u64(),
			"one hull, one seed, one stream"
		);

		let mut elsewhere = World::new(0x00C0_FFEE);
		assert_ne!(
			a.rng.next_u64(),
			elsewhere.rng.next_u64(),
			"a different seed is a different universe"
		);
	}
}
