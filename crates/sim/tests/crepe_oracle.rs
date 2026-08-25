//! The rules engine, checked against the walk.
//!
//! `energised_set` is a hand-rolled flood fill: a queue, a visited set, and a
//! `continue` for the open-breaker case. The doc's datalog section describes
//! exactly that code and claims two rules replace it — traversal, cycles,
//! termination and all.
//!
//! This file is that claim, tested. It does not replace the walk: the rail
//! says power's four roles are a five-line flood fill and the walk wins. What
//! it buys is a second opinion computed a completely different way, on a
//! question 34 other tests already pin down — the cheapest possible place to
//! learn whether the sandwich in the doc actually holds.
//!
//! If these two ever disagree, one of them is wrong about electricity and the
//! log will have been lying about which modules were live.

use std::collections::BTreeSet;

mod common;
use common::{TESTUDO, TESTUDO_2155, id, load, power_up};
use sim::{Command, ModuleId, World, tick};

/// Every module the sim believes is live, read through the public instrument.
/// `None` means the module has no opinion about electricity — a valve — and is
/// neither live nor dark, so it belongs in neither set.
fn walked(world: &World) -> BTreeSet<ModuleId> {
	world
		.modules()
		.keys()
		.copied()
		.filter(|id| world.is_energised(*id) == Some(true))
		.collect()
}

/// A hull with her battery on and both breakers shut.
fn lit(fixture: &str) -> World {
	let mut w = load(fixture);
	let script = power_up(&w);
	let _ = tick(&mut w, &script);
	w
}

#[test]
fn a_dark_ship_agrees_about_nothing() {
	// The trivial case, kept deliberately: two empty sets match, and that is
	// the correct answer rather than a vacuous one. It earns its place by
	// catching a projection that invents facts from an unpowered hull.
	let world = load(TESTUDO);

	assert!(walked(&world).is_empty(), "nothing is live on a cold hull");
	assert_eq!(reachability::solve(&world), walked(&world));
}

#[test]
fn the_rules_agree_with_the_walk() {
	// The case that means something. Both hulls, powered, with the assertion
	// that the answer is not empty — otherwise this is the test above wearing
	// a different name.
	for fixture in [TESTUDO, TESTUDO_2155] {
		let world = lit(fixture);
		let by_walk = walked(&world);

		assert!(
			by_walk.len() > 3,
			"a lit hull should have most of herself live"
		);
		assert_eq!(
			reachability::solve(&world),
			by_walk,
			"two rules and a flood fill must find the same ship"
		);
	}
}

#[test]
fn an_open_breaker_is_live_on_its_input_side() {
	// The case the doc's shorthand gets wrong. Its illustrative rule reads
	// `Energised(b) <- Energised(a), Wire(a, b), ClosedBreaker(b)` — gating on
	// the *destination*, which would make an open breaker dark. The walk gates
	// on the *origin*: it pushes a module into `reached` before asking whether
	// it conducts, so a breaker with live input is itself live and simply
	// passes nothing onward.
	//
	// The walk is right, and the difference is not pedantry: a breaker with
	// live input is exactly the thing that kills an electrician. A model that
	// calls it dark is a model that would let the deck report it as safe.
	let mut world = lit(TESTUDO);
	let brk = id(&world, "BRK-01");
	let lt = id(&world, "LT-01");

	let _ = tick(
		&mut world,
		&[Command::SetBreaker {
			id: brk,
			closed: false,
		}],
	);

	let by_rules = reachability::solve(&world);

	assert!(
		by_rules.contains(&brk),
		"an open breaker still has live input"
	);
	assert!(
		!by_rules.contains(&lt),
		"but the galley behind it goes dark"
	);
	assert_eq!(by_rules, walked(&world), "and the walk says the same");
}

#[test]
fn the_rules_track_the_ship_through_a_script() {
	// One agreement is a coincidence; agreeing at every step of a script is a
	// property. Each command changes the graph — a source dropped, a breaker
	// opened, both restored — and the two answers are compared after each.
	let world = load(TESTUDO);
	let (pwr, brk_1, brk_2) = (
		id(&world, "PWR-01"),
		id(&world, "BRK-01"),
		id(&world, "BRK-02"),
	);

	let script = [
		Command::SetSource {
			id: pwr,
			online: true,
		},
		Command::SetBreaker {
			id: brk_1,
			closed: true,
		},
		Command::SetBreaker {
			id: brk_2,
			closed: true,
		},
		Command::SetBreaker {
			id: brk_1,
			closed: false,
		},
		Command::SetSource {
			id: pwr,
			online: false,
		},
		Command::SetBreaker {
			id: brk_1,
			closed: true,
		},
		Command::SetSource {
			id: pwr,
			online: true,
		},
	];

	let mut world = world;
	for (step, command) in script.into_iter().enumerate() {
		let _ = tick(&mut world, &[command]);
		assert_eq!(
			reachability::solve(&world),
			walked(&world),
			"step {step} parted the rules from the walk"
		);
	}
}

/// Layer 1 and 2 of the doc's sandwich: `&World` projected into facts, then
/// solved to a fixpoint. There is no layer 3 or 4 here — the oracle concludes
/// and stops. Nothing it derives is allowed to mutate anything.
#[allow(
	clippy::disallowed_types,
	reason = "crepe's generated fixpoint uses HashSet/HashMap internally. a datalog \
	          solution is a SET — derivation order cannot change its contents — and \
	          solve() collects into a BTreeSet before anything can observe an order. \
	          the law holds where it matters: no hash iteration order reaches the log."
)]
mod reachability {
	use std::collections::BTreeSet;

	use crepe::crepe;
	use sim::{ModuleId, NetworkKind, World};

	crepe! {
		@input struct Source(ModuleId);
		@input struct Wire(ModuleId, ModuleId);
		@input struct Conducting(ModuleId);

		@output struct Energised(ModuleId);

		// a live source is live by being one. this is the paracausal case:
		// nothing upstream explains it.
		Energised(m) <- Source(m);

		// and current spreads along a wire out of anything live that conducts.
		// `Conducting(a)` gates on the ORIGIN, not the destination — that is
		// what makes an open breaker live on its input and dead downstream.
		Energised(b) <- Energised(a), Conducting(a), Wire(a, b);
	}

	/// Project a world into facts, solve, and hand back the conclusion as an
	/// ordered set.
	pub fn solve(world: &World) -> BTreeSet<ModuleId> {
		let mut runtime = Crepe::new();

		runtime.extend(
			world
				.modules()
				.keys()
				.copied()
				.filter(|id| world.source_online(*id) == Some(true))
				.map(Source),
		);
		runtime.extend(
			world
				.connections()
				.iter()
				.filter(|c| c.net == NetworkKind::Power)
				.map(|c| Wire(c.from.0, c.to.0)),
		);
		// an open breaker is not an `if` — it is a fact declined. `None` here
		// means "not a breaker", which conducts freely; only `Some(false)`
		// blocks, so absence from this relation means non-conducting.
		runtime.extend(
			world
				.modules()
				.keys()
				.copied()
				.filter(|id| world.breaker_closed(*id) != Some(false))
				.map(Conducting),
		);

		let (energised,) = runtime.run();
		energised.into_iter().map(|Energised(m)| m).collect()
	}
}
