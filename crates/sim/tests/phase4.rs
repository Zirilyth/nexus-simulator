//! Phase 4: condition has consequences.
//!
//! A worn part carries less than its nameplate says, and the gap between the
//! two is the diagnosis. These tests pin the arithmetic of that gap, the order
//! blame is assigned in, and — deliberately — the fact that a fault's cause
//! chain never names the fault.

mod common;
use common::{TESTUDO, TESTUDO_2155, degrade, id, load, power_up};
use sim::{Command, EventKind, ModuleId, Symptom, World, replay, tick};

/// Degrade something, then bring her up, in one batch. Commands settle in
/// order, so the fault is already in place when the ship energises.
fn degrade_then_power_up(world: &World, label: &str, to: f32) -> Vec<Command> {
	let mut script = vec![degrade(world, label, to)];
	script.extend(power_up(world));
	script
}

#[test]
fn a_tired_breaker_trips_early() {
	// BRK-01 carries 12+6+0+2+2+1 = 23A. Nameplate 25A, so healthy she holds.
	// At condition 0.5 her effective rating is 25 × (0.5/0.7) = 17.85 → 17A,
	// and the same 23A that was fine a moment ago takes her out.
	let mut w = load(TESTUDO);
	let brk01 = id(&w, "BRK-01");
	let script = degrade_then_power_up(&w, "BRK-01", 0.5);
	let events = tick(&mut w, &script);

	let tripped = events.iter().any(|e| {
		e.kind
			== EventKind::BreakerTripped {
				id: brk01,
				load_a: 23,
				rating_a: 25,
				degraded_rating_a: 17,
			}
	});
	assert!(tripped, "23A through a 17A-effective breaker: {events:#?}");

	// the control: the identical ship, no fault seeded, does not trip
	let mut healthy = load(TESTUDO);
	let script = power_up(&healthy);
	let events = tick(&mut healthy, &script);
	assert!(
		!events
			.iter()
			.any(|e| matches!(e.kind, EventKind::BreakerTripped { .. })),
		"23A on a 25A breaker is fine when the breaker is fine"
	);
}

#[test]
fn dead_modules_draw_nothing() {
	// HTR-01 is 12 of BRK-01's 23A. Dead, she pulls nothing: the breaker sees
	// 11A and the ship burns 17A instead of 29A. A failed heater is not
	// heating AND not eating — she stays energised, she just stops drawing.
	let mut w = load(TESTUDO);
	let pwr = id(&w, "PWR-01");
	let mut script = vec![degrade(&w, "HTR-01", 0.0), degrade(&w, "BRK-01", 0.5)];
	script.extend(power_up(&w));
	let events = tick(&mut w, &script);

	assert_eq!(
		w.charge_of(pwr),
		Some(144_000 - 17),
		"29A less the heater's 12"
	);
	assert!(
		!events
			.iter()
			.any(|e| matches!(e.kind, EventKind::BreakerTripped { .. })),
		"11A through a 17A-effective breaker holds: {events:#?}"
	);

	// the control: same tired breaker, live heater, and the 12A puts her over
	let mut w = load(TESTUDO);
	let script = degrade_then_power_up(&w, "BRK-01", 0.5);
	let events = tick(&mut w, &script);
	assert!(
		events
			.iter()
			.any(|e| matches!(e.kind, EventKind::BreakerTripped { .. })),
		"the heater's draw is the only thing that changed"
	);
}

#[test]
fn the_bus_gives_out() {
	// The whole ship is 29A. PWR-02 is rated 40A, so she carries it easily —
	// until condition 0.4 drops her to 40 × (0.4/0.7) = 22.85 → 22A.
	// No breaker is over its own rating here: this is the bus alone failing.
	let mut w = load(TESTUDO);
	let pwr02 = id(&w, "PWR-02");
	let script = degrade_then_power_up(&w, "PWR-02", 0.4);
	let first = tick(&mut w, &script);

	let exceeded = first.iter().any(|e| {
		e.kind
			== EventKind::CapacityExceeded {
				id: pwr02,
				load_a: 29,
				rating_a: 40,
				degraded_rating_a: 22,
			}
	});
	assert!(exceeded, "29A through a 22A-effective bus: {first:#?}");

	// scheduled, not instant — the breakers she feeds are still closed
	assert_eq!(
		w.breaker_closed(id(&w, "BRK-01")),
		Some(true),
		"thermal delay"
	);
	assert_eq!(
		w.breaker_closed(id(&w, "BRK-02")),
		Some(true),
		"thermal delay"
	);

	let _ = tick(&mut w, &[]);
	for brk in ["BRK-01", "BRK-02"] {
		assert_eq!(w.breaker_closed(id(&w, brk)), Some(false), "{brk} opens");
	}
	// a bus giving out takes both halves of the ship, not one
	for dark in [
		"HTR-01", "LSS-01", "LT-01", "NAV-01", "DCK-01", "PMP-01", "LT-02",
	] {
		let m = id(&w, dark);
		assert_eq!(w.is_energised(m), Some(false), "{dark}");
	}
}

#[test]
fn suppliers_outrank_breakers() {
	// The 2155 hull draws 33A, and BRK-01 already carries 27A against a 25A
	// nameplate — she would trip on her own. Degrade the bus to 0.5 and it is
	// over too, at 28A effective. Both faults are live in the same settle.
	//
	// The bus wins. Blaming the breaker for a bus failure sends the player
	// looking in the wrong place, which is a lie rather than a puzzle.
	let mut w = load(TESTUDO_2155);
	let pwr02 = id(&w, "PWR-02");
	let script = degrade_then_power_up(&w, "PWR-02", 0.5);
	let events = tick(&mut w, &script);

	let exceeded = events.iter().any(|e| {
		e.kind
			== EventKind::CapacityExceeded {
				id: pwr02,
				load_a: 33,
				rating_a: 40,
				degraded_rating_a: 28,
			}
	});
	assert!(exceeded, "the bus is the upstream truth: {events:#?}");
	assert!(
		!events
			.iter()
			.any(|e| matches!(e.kind, EventKind::BreakerTripped { .. })),
		"BRK-01 is over its rating too, but the bus claimed her first: {events:#?}"
	);
}

#[test]
fn faults_have_causes() {
	// Every consequence walks back to something paracausal — but NOT to the
	// fault itself. The chain names the load that pushed the breaker over,
	// because that is what physically happened. Noticing that 25 and 17
	// disagree is the player's job, and this test exists so nobody "fixes"
	// the chain into handing it over.
	let mut w = load(TESTUDO);
	let script = degrade_then_power_up(&w, "BRK-01", 0.5);
	let _ = tick(&mut w, &script);
	let _ = tick(&mut w, &[]);

	let lt01 = id(&w, "LT-01");
	let went_dark = w
		.log()
		.iter()
		.find(|e| {
			e.kind
				== EventKind::PowerChanged {
					id: lt01,
					energised: false,
				}
		})
		.expect("the seeded fault should darken LT-01");

	let mut at = went_dark;
	let mut hops = 0;
	let mut saw_condition_change = false;
	while let Some(cause) = at.cause {
		at = w
			.log()
			.iter()
			.find(|e| e.id == cause)
			.expect("a cause must name an event that exists");
		if matches!(at.kind, EventKind::ConditionChanged { .. }) {
			saw_condition_change = true;
		}
		hops += 1;
		assert!(hops < w.log().len(), "cause chain must not cycle");
	}

	assert!(hops > 0, "a dark lamp is nobody's spontaneous idea");
	assert_eq!(at.cause, None, "chain must bottom out at the paracausal");
	assert!(
		!saw_condition_change,
		"the cascade must not hand over the culprit — that gap is the game"
	);
}

#[test]
fn seeded_faults_replay() {
	// A seeded fault is a command, so a bug report is a script file.
	let text = std::fs::read_to_string(TESTUDO).expect("fixture should be readable");
	let mut live = World::from_fixture_str(&text).expect("testudo should load");
	let script = vec![degrade_then_power_up(&live, "BRK-01", 0.5), vec![], vec![]];
	for batch in &script {
		let _ = tick(&mut live, batch);
	}

	let refolded = replay(&text, &script).expect("replay should load the same hull");
	assert_eq!(live, refolded, "same seed, same script, same universe");
	assert!(
		live.log()
			.iter()
			.any(|e| matches!(e.kind, EventKind::ConditionChanged { .. })),
		"the script must actually contain the fault it claims to replay"
	);
}

#[test]
fn seeding_a_ghost_is_history_not_an_error() {
	// The boundary is total: a condition command aimed at nothing aboard is a
	// rejection in the log, never a panic.
	let mut w = load(TESTUDO);
	let events = tick(
		&mut w,
		&[Command::SetCondition {
			id: ModuleId(999),
			new_condition: sim::Condition::new(0.5).expect("0.5 is a condition"),
		}],
	);

	assert_eq!(events.len(), 1);
	assert!(
		matches!(events[0].kind, EventKind::CommandRejected { .. }),
		"{:?}",
		events[0].kind
	);
	assert_eq!(events[0].cause, None, "rejections are paracausal");
}

// ---- scan: the instrument -------------------------------------------------
//
// A symptom is observable AT the module reported. It never names another
// module, a rating, a condition, or a cause. These tests pin that, and pin
// the threshold that decides what "close to the limit" means — because a
// threshold set slightly wrong makes a healthy ship report sick, and every
// other scan assertion would still pass.

/// Every module that has an opinion about electricity, and what it reports.
fn symptoms(world: &World) -> Vec<(String, Symptom)> {
	world
		.modules()
		.iter()
		.filter_map(|(id, meta)| Some((meta.label.clone(), world.symptom_of(*id)?)))
		.collect()
}

#[test]
fn a_healthy_ship_reports_nothing() {
	// 29A of ship against a 40A bus is 72.5% — under the 80% threshold, so a
	// sound testudo reads all-nominal. That margin is what this test pins: drop
	// the threshold to 70% and a working ship starts crying wolf, while every
	// other scan test below would still pass.
	let mut w = load(TESTUDO);

	for (label, symptom) in symptoms(&w) {
		assert_eq!(symptom, Symptom::Dark, "{label} before power-up");
	}

	let script = power_up(&w);
	let _ = tick(&mut w, &script);

	for (label, symptom) in symptoms(&w) {
		assert_eq!(symptom, Symptom::Nominal, "{label} on a sound ship");
	}
}

#[test]
fn valves_have_no_opinion_about_electricity() {
	// None is not a symptom. A valve is aboard and observable, just not on this
	// net — it gets one in phase 6 and scan will report it then.
	let w = load(TESTUDO);
	for label in ["V-01", "V-02"] {
		assert_eq!(w.symptom_of(id(&w, label)), None, "{label}");
	}
	assert!(
		w.symptom_of(id(&w, "LT-01")).is_some(),
		"a lamp is on the power net"
	);
}

#[test]
fn a_sagging_bus_starves_what_it_feeds() {
	// PWR-02 at 0.6 carries 40 × (0.6/0.7) = 34A effective against 29A of ship:
	// 85%, over the threshold, under the limit. Nothing faults, nothing trips,
	// the log stays quiet — and ten modules report trouble.
	let mut w = load(TESTUDO);
	let script = degrade_then_power_up(&w, "PWR-02", 0.6);
	let events = tick(&mut w, &script);

	assert!(
		!events.iter().any(|e| matches!(
			e.kind,
			EventKind::CapacityExceeded { .. } | EventKind::BreakerTripped { .. }
		)),
		"sagging is not failing: the ship still runs. {events:#?}"
	);

	// the culprit reports nothing wrong. it is not BEING starved — it is the
	// one struggling to supply, and the instrument points away from it.
	assert_eq!(w.symptom_of(id(&w, "PWR-02")), Some(Symptom::Nominal));
	assert_eq!(w.symptom_of(id(&w, "PWR-01")), Some(Symptom::Nominal));

	// everything it feeds, across BOTH breakers — the intersection is the clue
	for label in [
		"BRK-01", "BRK-02", "LSS-01", "HTR-01", "PMP-01", "SNS-01", "LT-01", "LT-02", "NAV-01",
		"DCK-01",
	] {
		assert_eq!(
			w.symptom_of(id(&w, label)),
			Some(Symptom::Starved),
			"{label} is fed by the sagging bus"
		);
	}
}

#[test]
fn dark_beats_starved() {
	// Reaching this needs care: after a breaker opens, the load usually drops
	// too far for any bus that survived the full load to still be sagging. So
	// bring her up with BRK-01 already open — the bus only ever sees BRK-02's
	// 6A, and at condition 0.12 its effective limit is 6A. Sagging, not
	// faulting, while half the ship was never energised at all.
	let mut w = load(TESTUDO);
	let script = vec![
		degrade(&w, "PWR-02", 0.12),
		Command::SetBreaker {
			id: id(&w, "BRK-01"),
			closed: false,
		},
		Command::SetSource {
			id: id(&w, "PWR-01"),
			online: true,
		},
	];
	let _ = tick(&mut w, &script);

	// LT-01 is downstream of the sagging bus AND unpowered. Dark wins: asking
	// about supply quality at something with no supply is nonsense.
	assert_eq!(w.symptom_of(id(&w, "LT-01")), Some(Symptom::Dark));
	// its sibling on the live breaker reports the sag, proving the bus really
	// is in the sagging set and LT-01 was not merely missed
	assert_eq!(w.symptom_of(id(&w, "LT-02")), Some(Symptom::Starved));
}
