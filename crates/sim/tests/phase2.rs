//! Phase 2's definition of done: the five tests from the build sheet.
//!
//! These drive the sim the only way anything is allowed to — commands in,
//! events out. If one of these ever needs to reach into `World` and change
//! something, the boundary has been broken and the test is the evidence.

use sim::{Command, Event, EventKind, ModuleId, World, tick};

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

fn energised_in(events: &[Event], who: ModuleId) -> bool {
	events.iter().any(|e| {
		e.kind
			== EventKind::PowerChanged {
				id: who,
				energised: true,
			}
	})
}

fn went_dark_in(events: &[Event], who: ModuleId) -> bool {
	events.iter().any(|e| {
		e.kind
			== EventKind::PowerChanged {
				id: who,
				energised: false,
			}
	})
}

#[test]
fn power_reaches_the_lights() {
	let mut w = load(TESTUDO);
	let lt01 = id(&w, "LT-01");
	assert_eq!(w.is_energised(lt01), Some(false), "dead ship at load");

	let script = power_up(&w);
	let events = tick(&mut w, &script);

	assert!(
		energised_in(&events, lt01),
		"LT-01 should announce itself: {events:#?}"
	);
	assert_eq!(w.is_energised(lt01), Some(true));
	// two connections from the battery, and it still hears about it
	assert_eq!(w.is_energised(id(&w, "NAV-01")), Some(true));
	assert_eq!(w.is_energised(id(&w, "PMP-01")), Some(true));
}

#[test]
fn breaker_isolates() {
	let mut w = load(TESTUDO);
	let script = power_up(&w);
	let _ = tick(&mut w, &script); // setup: the log is what these read

	let brk01 = id(&w, "BRK-01");
	let events = tick(
		&mut w,
		&[Command::SetBreaker {
			id: brk01,
			closed: false,
		}],
	);

	for dark in ["HTR-01", "LSS-01", "LT-01", "NAV-01", "DCK-01"] {
		let m = id(&w, dark);
		assert!(went_dark_in(&events, m), "{dark} should go dark");
		assert_eq!(w.is_energised(m), Some(false), "{dark}");
	}

	// the other half of the ship never noticed
	for lit in ["PMP-01", "LT-02"] {
		let m = id(&w, lit);
		assert!(!went_dark_in(&events, m), "{lit} is on BRK-02, leave it be");
		assert_eq!(w.is_energised(m), Some(true), "{lit}");
	}
}

#[test]
fn the_galley_trap() {
	let mut w = load(TESTUDO_2155);
	let script = power_up(&w);
	let brk01 = id(&w, "BRK-01");

	let first = tick(&mut w, &script);
	let tripped = first.iter().any(|e| {
		e.kind
			== EventKind::BreakerTripped {
				id: brk01,
				load_a: 27,
				rating_a: 25,
			}
	});
	assert!(tripped, "27A on a 25A breaker: {first:#?}");

	// scheduled, not instant — she is still closed the tick she trips
	assert_eq!(w.breaker_closed(brk01), Some(true), "thermal delay");
	assert_eq!(w.is_energised(id(&w, "LT-01")), Some(true));

	let second = tick(&mut w, &[]);
	assert_eq!(w.breaker_closed(brk01), Some(false), "opens the next tick");
	for dark in ["HTR-01", "LSS-01", "LT-01", "NAV-01", "DCK-01"] {
		let m = id(&w, dark);
		assert!(went_dark_in(&second, m), "{dark} should go dark");
	}
	assert_eq!(w.is_energised(id(&w, "PMP-01")), Some(true), "BRK-02 holds");
}

#[test]
fn trips_have_causes() {
	let mut w = load(TESTUDO_2155);
	let script = power_up(&w);
	let _ = tick(&mut w, &script); // setup: the log is what these read
	let _ = tick(&mut w, &[]);

	let trip = w
		.log
		.iter()
		.find(|e| matches!(e.kind, EventKind::BreakerTripped { .. }))
		.expect("the galley trap trips");

	// walk the chain back. it must terminate, and it must terminate at the
	// paracausal: a command, or the world coming into existence.
	let mut at = trip;
	let mut hops = 0;
	while let Some(cause) = at.cause {
		at = w
			.log
			.iter()
			.find(|e| e.id == cause)
			.expect("a cause must name an event that exists");
		hops += 1;
		assert!(hops < w.log.len(), "cause chain must not cycle");
	}

	assert!(hops > 0, "a trip is nobody's spontaneous idea");
	assert!(
		matches!(
			at.kind,
			EventKind::WorldLoaded | EventKind::SourceSet { .. } | EventKind::BreakerSet { .. }
		),
		"chain must bottom out at the paracausal, got {:?}",
		at.kind
	);
}

#[test]
fn canary_v2() {
	// same fixture, same script, two universes. byte-identical history.
	// this test never gets weakened and never gets deleted.
	let script = |w: &World| {
		vec![
			power_up(w),
			vec![],
			vec![Command::SetBreaker {
				id: id(w, "BRK-02"),
				closed: false,
			}],
			vec![],
		]
	};

	let mut a = load(TESTUDO_2155);
	let mut b = load(TESTUDO_2155);
	for batch in script(&a) {
		let _ = tick(&mut a, &batch);
	}
	for batch in script(&b) {
		let _ = tick(&mut b, &batch);
	}

	assert_eq!(a.log, b.log, "same seed, same script, same history");
	assert_eq!(a.tick, b.tick);
	assert!(
		a.log
			.iter()
			.any(|e| matches!(e.kind, EventKind::BreakerTripped { .. })),
		"the canary must actually sing — script should include a trip"
	);
}
