pub mod types;
use crate::systems::commands::apply_commands;
pub use types::events::{Command, Event, EventId, EventKind};
pub use types::modules::{ModuleId, ModuleKind, ModuleMeta};
pub use types::world::{Connection, LoadError, World};
pub mod systems;

#[must_use = "events are history. drop them deliberately or not at all"]
pub fn add(left: u64, right: u64) -> u64 {
	left + right
}
#[must_use = "events are history. drop them deliberately or not at all"]
pub fn hello() -> &'static str {
	"deck online"
}
#[must_use = "events are history. drop them deliberately or not at all"]
pub fn tick(world: &mut World, commands: &[Command]) -> Vec<Event> {
	let first_new = world.log.len();

	// trips scheduled last tick open first: a thermal breaker acts late, and
	// the delay is what keeps a trip from feeding back into its own cause.
	crate::systems::power::tick_power(world);
	crate::systems::power::tick_depletion(world);
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

#[cfg(test)]
mod tests {
	use crate::types::modules::ModuleId;

	use super::*;
	use rand_chacha::rand_core::RngCore;

	#[test]
	fn it_works() {
		let result = add(2, 2);
		assert_eq!(result, 4);
	}

	#[test]
	fn same_seed_same_universe() {
		let mut a = World::new(0x00C0_FFEE);
		let mut b = World::new(0x00C0_FFEE);
		assert_eq!(a.rng.next_u64(), b.rng.next_u64());

		let ea = crate::tick(
			&mut a,
			&[Command::SetBreaker {
				id: ModuleId(2),
				closed: true,
			}],
		);
		let eb = crate::tick(
			&mut b,
			&[Command::SetBreaker {
				id: ModuleId(2),
				closed: true,
			}],
		);

		assert_eq!(ea, eb);
		assert_eq!(a.tick, b.tick);
	}
}
