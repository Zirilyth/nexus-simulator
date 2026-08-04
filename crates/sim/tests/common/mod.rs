use sim::{Command, ModuleId, World};

pub const TESTUDO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/testudo.ron");
pub const TESTUDO_2155: &str = concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/../../fixtures/testudo-2155.ron"
);

pub fn load(path: &str) -> World {
	World::from_fixture_file(path).expect("fixture should load")
}

pub fn id(world: &World, label: &str) -> ModuleId {
	world.find_label(label).expect("label should be aboard")
}

/// The script that brings a testudo up: battery online, both breakers shut.
pub fn power_up(world: &World) -> Vec<Command> {
	vec![
		Command::SetSource {
			id: id(world, "PWR-01"),
			online: true,
		},
		Command::SetBreaker {
			id: id(world, "BRK-01"),
			closed: true,
		},
		Command::SetBreaker {
			id: id(world, "BRK-02"),
			closed: true,
		},
	]
}
