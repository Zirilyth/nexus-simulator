//! Phase 4.5: the universe learns to skip.
//!
//! `tick()` walks time. `advance_to()` jumps it. These tests exist to prove
//! those are the same universe — that skipping a decade in one call produces
//! byte-identical history to living through it a second at a time.
//!
//! This is `anchors_dont_drift` grown up: phase 3 proved charge was derived
//! rather than accumulated, so a jump could not lose it. This proves the same
//! of the whole sim, events and all.

mod common;
use common::{TESTUDO, degrade, id, load, power_up};
use sim::{EventKind, advance_to, tick};

/// Walk to `t` a tick at a time, the old way.
fn stepped(fixture: &str, script: &[sim::Command], t: u64) -> sim::World {
	let mut w = load(fixture);
	let _ = tick(&mut w, script);
	while w.tick() < t {
		let _ = tick(&mut w, &[]);
	}
	w
}

/// Jump straight to `t`, stopping only where something happens.
fn jumped(fixture: &str, script: &[sim::Command], t: u64) -> sim::World {
	let mut w = load(fixture);
	let _ = tick(&mut w, script);
	let _ = advance_to(&mut w, t);
	w
}

#[test]
fn jumping_and_stepping_are_the_same_universe() {
	// Past the battery's death at 4965, so the span contains a real event
	// rather than only silence. If the jump skipped it, or fired it at the
	// wrong tick, the logs diverge and this fails loudly.
	let w = load(TESTUDO);
	let script = power_up(&w);

	let walked = stepped(TESTUDO, &script, 6000);
	let leapt = jumped(TESTUDO, &script, 6000);

	assert_eq!(walked.log(), leapt.log(), "same history, either way");
	assert_eq!(walked.tick(), leapt.tick(), "both arrive at 6000");
	assert_eq!(walked, leapt, "same universe, whole and entire");

	// and the span was not empty — a test that passes on two empty logs
	// would prove nothing at all
	assert!(
		leapt
			.log()
			.iter()
			.any(|e| matches!(e.kind, EventKind::SourceDepleted { .. })),
		"the battery must actually die inside the jump"
	);
}

#[test]
fn a_jump_lands_on_the_scheduled_tick() {
	// The depletion is solved for tick 4965. Jumping past it must stamp the
	// event at 4965, not at the tick we happened to stop on.
	let w = load(TESTUDO);
	let script = power_up(&w);
	let leapt = jumped(TESTUDO, &script, 6000);

	let death = leapt
		.log()
		.iter()
		.find(|e| matches!(e.kind, EventKind::SourceDepleted { .. }))
		.expect("she dies inside the span");

	assert_eq!(
		death.at_tick, 4965,
		"events happen when they were scheduled"
	);
}

#[test]
fn a_thermal_delay_survives_being_jumped_over() {
	// A trip schedules its breaker to open one tick later. Stepping gives that
	// delay for free — the tick loop provides it. Jumping must honour it from
	// the stored fire time instead, so this is the test that catches the delay
	// being quietly deleted along with the loop.
	let w = load(TESTUDO);
	let mut script = vec![degrade(&w, "BRK-01", 0.5)];
	script.extend(power_up(&w));

	let walked = stepped(TESTUDO, &script, 40);
	let leapt = jumped(TESTUDO, &script, 40);

	assert_eq!(walked.log(), leapt.log(), "the delay is data, not timing");
	assert_eq!(
		walked.breaker_closed(id(&walked, "BRK-01")),
		Some(false),
		"BRK-01 opens either way"
	);
}

#[test]
fn a_quiet_world_costs_one_comparison() {
	// A dark hull has nothing scheduled, so a jump of any length is a single
	// question answered `None`. This is the assertion the 10^6 bet rests on:
	// a thousand derelicts drifting for a decade must cost nothing.
	let mut w = load(TESTUDO);
	let before = w.log().len();

	let events = advance_to(&mut w, 10_000_000);

	assert!(events.is_empty(), "nothing happens to a dead ship");
	assert_eq!(w.log().len(), before, "and nothing is written down");
	assert_eq!(w.tick(), 10_000_000, "but the clock still arrives");
}
