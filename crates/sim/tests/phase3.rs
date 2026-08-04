use sim::{Command, EventKind, ModuleId, World, replay, tick};

const TESTUDO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/testudo.ron");
const TESTUDO_2155: &str = concat!(
	env!("CARGO_MANIFEST_DIR"),
	"/../../fixtures/testudo-2155.ron"
);

fn load(path: &str) -> World {
	World::from_fixture_file(path).expect("fixture should load")
}

fn id(world: &World, label: &str) -> ModuleId {
	world.find_label(label).expect("label should be aboard")
}

/// The script that brings a testudo up: battery online, both breakers shut.
fn power_up(world: &World) -> Vec<Command> {
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

#[test]
fn the_battery_dies() {
	let mut w = load(TESTUDO);
	let pwr = id(&w, "PWR-01");
	let script = power_up(&w);
	let _ = tick(&mut w, &script);

	// one tick of burn already happened; charge is derived at the current tick
	assert_eq!(w.charge_of(pwr), Some(144_000 - 29));

	for _ in 0..7000 {
		let _ = tick(&mut w, &[]);
	}

	let death = w
		.log
		.iter()
		.find(|e| e.kind == EventKind::SourceDepleted { id: pwr })
		.expect("she must die");
	assert_eq!(w.charge_of(pwr), Some(0));
	assert_eq!(death.at_tick, 4965, "144_000 / 29, floored");

	for dark in ["LT-01", "HTR-01", "PMP-01", "NAV-01"] {
		assert_eq!(w.is_energised(id(&w, dark)), Some(false), "{dark}");
	}
}

#[test]
fn anchors_dont_drift() {
	let mut w = load(TESTUDO);
	let pwr = id(&w, "PWR-01");
	let script = power_up(&w);
	let _ = tick(&mut w, &script); // world.tick == 1
	for _ in 0..3999 {
		let _ = tick(&mut w, &[]); // world.tick == 4000
	}
	// the closed form, not whatever a loop accumulated
	assert_eq!(w.charge_of(pwr), Some(144_000 - 29 * 4000)); //28_000
}
#[test]
fn canary_v3() {
	// a trip AND a depletion, twice, byte-identical. never weakened, never deleted.
	let run = || {
		let mut w = load(TESTUDO_2155);
		let script = power_up(&w);
		let _ = tick(&mut w, &script);
		for _ in 0..24_000 {
			let _ = tick(&mut w, &[]);
		}
		w
	};
	let (a, b) = (run(), run());
	assert_eq!(a.log, b.log);
	assert!(
		a.log
			.iter()
			.any(|e| matches!(e.kind, EventKind::BreakerTripped { .. }))
	);
	assert!(
		a.log
			.iter()
			.any(|e| matches!(e.kind, EventKind::SourceDepleted { .. }))
	);
}
#[test]
fn replay_refolds() {
	let text = std::fs::read_to_string(TESTUDO).expect("fixture");
	let mut live = World::from_fixture_str(&text).expect("load");
	let script = vec![power_up(&live), vec![], vec![], vec![]];
	for batch in &script {
		let _ = tick(&mut live, batch);
	}
	let refolded = replay(&text, &script).expect("replay");
	assert_eq!(live, refolded, "same seed, same script, same universe");
}
