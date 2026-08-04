pub mod types;
pub use types::events::{Command, Event, EventId, EventKind};
pub use types::modules::{ModuleId, ModuleKind, ModuleMeta};
pub use types::world::{Connection, World};

use crate::systems::commands::apply_commands;
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
	apply_commands(world, commands);

	world.tick += 1;
	world.log[first_new..].to_vec()
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
