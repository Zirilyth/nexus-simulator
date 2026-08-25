//! Phase 4.5: many ships, one clock.
//!
//! A `Universe` is a set of independent worlds sharing a timeline. This file
//! owns the property that licenses everything built on top of it: a ship
//! advanced inside a universe must be exactly the ship it would have become
//! alone.
//!
//! That is not tidiness. It is the parallelism licence. If two undocked hulls
//! can influence each other, no amount of threading is safe later and the 10^6
//! bet is off. `phase45.rs` proved a jump equals a walk for one ship; this
//! proves a fleet equals its ships.

mod common;
use common::{TESTUDO, TESTUDO_2155, load, power_up};
use sim::{EventKind, ShipId, Universe, World, advance_to, tick};

/// How far to run. Past the testudo's depletion at 4965, so the span contains
/// real events rather than silence — two empty logs match trivially and would
/// prove nothing at all.
const HORIZON: u64 = 6000;

/// A hull brought up and left running, with no universe around it.
fn alone(fixture: &str) -> World {
	let mut w = powered(fixture);
	let _ = advance_to(&mut w, HORIZON);
	w
}

/// A hull brought up, but not yet advanced. Commands cross the boundary before
/// the ship joins a universe, because `Universe::advance_to` takes no script —
/// time is the only thing it moves.
fn powered(fixture: &str) -> World {
	let mut w = load(fixture);
	let script = power_up(&w);
	let _ = tick(&mut w, &script);
	w
}

/// Bring a hull up and hand it to a universe.
fn aboard(universe: &mut Universe, fixture: &str) -> ShipId {
	let w = powered(fixture);
	universe.add_world(w)
}

#[test]
fn a_ship_in_a_universe_is_the_ship_it_would_have_been_alone() {
	let mut universe = Universe::new();
	let testudo = aboard(&mut universe, TESTUDO);
	let derelict = aboard(&mut universe, TESTUDO_2155);

	universe.advance_to(HORIZON);

	// `World` derives `PartialEq`, so this compares the whole hull — log, tick,
	// charges, breaker states, every anchor. Not just the events that happened
	// to be visible.
	assert_eq!(
		universe.world(testudo),
		Some(&alone(TESTUDO)),
		"a hull is not changed by having neighbours"
	);
	assert_eq!(
		universe.world(derelict),
		Some(&alone(TESTUDO_2155)),
		"and neither is the hull next to it"
	);
}

#[test]
fn advancing_a_fleet_is_not_advancing_a_single_ship_twice() {
	// The test above would pass on two identical hulls even if the universe
	// were quietly running one ship and copying it. These two are different
	// vessels, so their histories must differ — which is what makes the
	// equality above mean something.
	let a = alone(TESTUDO);
	let b = alone(TESTUDO_2155);

	assert_ne!(a.log(), b.log(), "two hulls, two histories");
}

#[test]
fn the_jump_crossed_real_events() {
	// The guard against a vacuous pass. If nothing happens between tick 1 and
	// the horizon, every assertion in this file compares empty logs and holds
	// for the wrong reason.
	let mut universe = Universe::new();
	let testudo = aboard(&mut universe, TESTUDO);

	universe.advance_to(HORIZON);

	let ship = universe.world(testudo).expect("she is aboard");
	assert!(
		ship.log()
			.iter()
			.any(|e| matches!(e.kind, EventKind::SourceDepleted { .. })),
		"the battery must actually die inside the span"
	);
}

#[test]
fn a_universe_is_due_when_its_earliest_ship_is() {
	// Each level aggregates only the level below: a world asks its networks,
	// a universe asks its worlds. Nothing else is scheduled at this height.
	let mut universe = Universe::new();
	aboard(&mut universe, TESTUDO);
	aboard(&mut universe, TESTUDO_2155);

	let earliest = [powered(TESTUDO), powered(TESTUDO_2155)]
		.iter()
		.filter_map(World::next_event_at)
		.min();

	assert_eq!(
		universe.next_event_at(),
		earliest,
		"the fleet is due when the first ship is"
	);
	assert!(
		earliest.is_some_and(|t| t <= HORIZON),
		"and that is inside the horizon, or the tests above run on silence"
	);
}
