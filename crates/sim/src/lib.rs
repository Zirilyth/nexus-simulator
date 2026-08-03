pub mod types;

pub use types::events::{Command, Event, EventId, EventKind};
pub use types::ids::{ModuleId, NetworkKind, PortName};
pub use types::meta::{ModuleKind, ModuleMeta};
pub use types::world::{Connection, World};

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
	for _cmd in commands {}
	world.tick += 1;
	Vec::new()
}

#[cfg(test)]
mod tests {
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

		let ea = crate::tick(&mut a, &[Command::Wait]);
		let eb = crate::tick(&mut b, &[Command::Wait]);

		assert_eq!(ea, eb);
		assert_eq!(a.tick, b.tick);
	}
}
