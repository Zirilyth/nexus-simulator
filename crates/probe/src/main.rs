//! The 10^6 probe — phase 4.5, step 4.
//!
//! The bet, stated plainly: N simulated objects for X ticks, where N and X are
//! both past a million, and it still runs. Everything before this was
//! mechanism. This is the measurement that says whether the mechanism scales
//! or whether the architecture has to change.
//!
//! It lives outside `sim` on purpose. The sim has one clock — `world.tick` —
//! and `clippy.toml` denies `Instant::now` by path to keep it that way.
//! Measuring needs a wall-clock, so the wall-clock lives out here, behind one
//! function with one `allow`, and reaches the sim only through its public API.
//! The instrument is not allowed inside the thing it measures.
//!
//! Usage: `cargo run --release -p probe -- [hulls] [modules] [active%] [horizon] [lookahead]`
//! Release matters. A debug build measures LLVM's day off, not yours.
#![allow(clippy::print_stdout, reason = "a measuring instrument reports")]
#![allow(
	clippy::cast_precision_loss,
	clippy::cast_possible_truncation,
	clippy::as_conversions,
	clippy::arithmetic_side_effects,
	clippy::expect_used,
	reason = "reporting arithmetic — a tenth of a microsecond either way is not the finding. \
	          the sim's laws bind the sim; this is the instrument reading it from outside."
)]

use sim::{Command, Universe, World, tick};
use std::fmt::Write as _;
use std::time::{Duration, Instant};

/// Loads hung behind one breaker. Keeps every breaker under its rating, so
/// nothing trips and the measurement is of steady state rather than cascade.
const LOADS_PER_BREAKER: usize = 20;

/// Every load draws this. One amp keeps the arithmetic legible: total draw in
/// amps is the number of loads, and depletion falls where we put it.
const DRAW_PER_LOAD: u32 = 1;

/// Where a powered hull's battery dies. Far enough in that the run has to
/// cross it, close enough that it lands inside a sane horizon.
const DEPLETE_AT: u64 = 5_000;

fn main() {
	let mut args = std::env::args().skip(1);
	let hulls = arg(&mut args, 2_000);
	let modules = arg(&mut args, 500);
	let active_pct = arg(&mut args, 10);
	let horizon = arg(&mut args, 20_000) as u64;
	let lookahead = arg(&mut args, 0) as u64;

	let (breakers, loads) = layout(modules);
	let per_hull = 2 + breakers + loads;
	let total = hulls * per_hull;
	let active = hulls * active_pct / 100;

	println!("probe — {hulls} hulls x {per_hull} modules = {total} objects");
	println!(
		"  {active} powered ({active_pct}%), {} dark",
		hulls - active
	);
	println!("  horizon {horizon} ticks, batteries die at {DEPLETE_AT}");
	match lookahead {
		0 => println!("  no interaction — one jump, undocked hulls"),
		d => println!("  lookahead {d} ticks — the fleet syncs every {d}"),
	}
	println!();

	// ---- 1. build ------------------------------------------------------
	// Fixture text in, World out. This is parse + resolve + index, and it is
	// where a million heap-allocated labels would show up if they are going to.
	let rss_before = rss_kb();
	let ((universe, ids), build) = timed(|| build(hulls, modules));
	let rss_after = rss_kb();

	let held = rss_after.saturating_sub(rss_before);
	report("build", build);
	println!("    {:>10.1} us / hull", per_unit(build, hulls));
	println!("    {:>10.2} us / module", per_unit(build, total));
	println!(
		"    {held:>10} KiB resident  ({:.0} B / module)",
		bytes_per(held, total)
	);
	println!();

	// ---- 2. one settle -------------------------------------------------
	// Bringing a hull up is one command batch and one settle over its whole
	// power graph. Timed on a single hull, so this is the per-ship cost that
	// everything else multiplies.
	let mut universe = universe;
	let (settle, powered, settles) = power_up(&mut universe, &ids, active);
	report("power-up", settle);
	println!(
		"    {settles:>10} settles ({} per hull — one per accepted command)",
		settles / active.max(1)
	);
	println!("    {:>10.2} us / settle", per_unit(settle, settles));
	println!(
		"    {:>10.3} us / module / settle",
		per_unit(settle, powered * settles / active.max(1))
	);
	println!();

	// ---- 3. the jump ---------------------------------------------------
	// The whole bet. Most hulls have nothing scheduled, so `advance_to` should
	// ask each one a question, hear `None`, and move on. If this tracks the
	// number of *powered* hulls, the architecture holds. If it tracks the
	// number of hulls, the per-world scan is the ceiling and the priority
	// queue stops being premature.
	let (barriers, jump) = timed(|| advance_windowed(&mut universe, horizon, lookahead));
	report("advance", jump);
	println!("    {barriers:>10} barriers");
	println!(
		"    {:>10.3} ms / barrier",
		jump.as_secs_f64() * 1000.0 / barriers as f64
	);
	println!(
		"    {:>10.2} ns / hull asked",
		jump.as_nanos() as f64 / (hulls * barriers) as f64
	);

	let events: usize = ids
		.iter()
		.filter_map(|id| universe.world(*id))
		.map(|w| w.log().len())
		.sum();
	println!("    {events:>10} events in the log");
	if jump.as_secs_f64() > 0.0 {
		println!(
			"    {:>10.0} events / sec",
			events as f64 / jump.as_secs_f64()
		);
	}
	println!();
	println!("prediction: `advance` tracks powered hulls, not total hulls.");
	println!("if it tracks total, next_event_at's O(worlds) scan is the ceiling.");
	println!("with a lookahead, cost should scale as horizon/lookahead — halve the");
	println!("latency, double the barriers. a zero-latency channel is the cliff.");
}

/// Advance to `horizon` in windows of `lookahead` ticks, returning how many
/// barriers that took.
///
/// A window is a synchronisation barrier. Once ships can send each other
/// messages, no world may be advanced past the point where one could arrive,
/// so the fleet can only move in steps of the shortest channel latency —
/// that is what lookahead means in a parallel discrete-event simulation, and
/// it is why every cross-ship channel must declare a latency above zero.
///
/// `lookahead == 0` is the no-interaction case: one jump, which is how
/// undocked hulls run today.
///
/// Nothing here sends a message — the sim has no comms yet. What it measures
/// is the *synchronisation* cost of having them: the price of asking every
/// world whether it is due, once per barrier. That term scales as
/// `horizon / lookahead` and it is the one that bites. The per-message settle
/// cost is already known from the power-up figure above.
fn advance_windowed(universe: &mut Universe, horizon: u64, lookahead: u64) -> usize {
	if lookahead == 0 {
		universe.advance_to(horizon);
		return 1;
	}
	let mut at = 0;
	let mut barriers = 0;
	while at < horizon {
		at = (at + lookahead).min(horizon);
		universe.advance_to(at);
		barriers += 1;
	}
	barriers
}

/// Build the fleet. Every hull is identical but for its seed, because the
/// question here is scale, not variety — a thousand different ships would
/// measure the same code paths and make the numbers harder to read.
fn build(hulls: usize, modules: usize) -> (Universe, Vec<sim::ShipId>) {
	let mut universe = Universe::new();
	let mut ids = Vec::with_capacity(hulls);
	for i in 0..hulls {
		let text = hull_ron(0x00C0_FFEE + i as u64, modules);
		let world = World::from_fixture_str(&text).expect("synthetic hull should load");
		ids.push(universe.add_world(world));
	}
	(universe, ids)
}

/// Bring the first `active` hulls online: battery on, every breaker shut.
/// Returns the time spent and how many modules were settled, so the per-module
/// figure divides by work done rather than by fleet size.
fn power_up(
	universe: &mut Universe,
	ids: &[sim::ShipId],
	active: usize,
) -> (Duration, usize, usize) {
	let mut spent = Duration::ZERO;
	let mut settled = 0;
	let mut settles = 0;
	for id in ids.iter().take(active) {
		let Some(world) = universe.world_mut(*id) else {
			continue;
		};
		let script = bring_up(world);
		settled += world.modules().len();
		// every accepted command settles the whole graph, so a batch of n
		// commands is n settles — not one. that is the number that matters.
		settles += script.len();
		let (_, d) = timed(|| tick(world, &script));
		spent += d;
	}
	(spent, settled, settles)
}

/// The script that wakes a synthetic hull. Labels are read back out of the
/// world rather than assumed, so this does not quietly depend on the order the
/// fixture happened to list them in.
fn bring_up(world: &World) -> Vec<Command> {
	world
		.modules()
		.iter()
		.filter_map(|(id, m)| match m.label.as_bytes() {
			[b'P', b'W', b'R', ..] => Some(Command::SetSource {
				id: *id,
				online: true,
			}),
			[b'B', b'R', b'K', ..] => Some(Command::SetBreaker {
				id: *id,
				closed: true,
			}),
			_ => None,
		})
		.collect()
}

/// How many breakers and loads make up a hull of `target` modules, after the
/// battery and the bus have taken their two. Loads spread evenly, so no
/// breaker carries more than `LOADS_PER_BREAKER` and nothing trips.
fn layout(target: usize) -> (usize, usize) {
	let body = target.saturating_sub(2).max(2);
	let breakers = body.div_ceil(LOADS_PER_BREAKER.saturating_add(1)).max(1);
	(breakers, body - breakers)
}

/// A hull as fixture text: one battery, one bus, a rank of breakers, and a
/// load behind each one. Generated rather than authored because the shape does
/// not matter here — only how many of it there are.
fn hull_ron(seed: u64, target: usize) -> String {
	let (breakers, loads) = layout(target);
	let draw = loads as u64 * u64::from(DRAW_PER_LOAD);

	// capacity is amp-ticks: enough to run the whole hull until DEPLETE_AT and
	// not one tick longer, so every powered ship has exactly one scheduled
	// event and we know where it is.
	let capacity = draw * DEPLETE_AT;
	let headroom = (draw as u32).saturating_add(10);

	let mut s = String::with_capacity(target * 128);
	s.push_str("ShipFixture(\n");
	let _ = writeln!(s, "class: \"probe\", hull_no: \"SYN-{seed:X}\",");
	let _ = writeln!(s, "commissioned: 2150.0, seed: {seed},");
	s.push_str("rooms: [ (id: \"R1\", name: \"hold\") ],\n");
	s.push_str("runs: [ (id: \"TRUNK\", cap: 999999) ],\n");

	s.push_str("parts: [\n");
	let _ = writeln!(
		s,
		"(id: \"CELL\", name: \"battery\", manufacturer: \"Synth\", manufacturer_part_number: \"CELL\", power: Some(Source(capacity: {capacity}, max_draw_a: {headroom}))),"
	);
	let _ = writeln!(
		s,
		"(id: \"BUS\", name: \"bus\", manufacturer: \"Synth\", manufacturer_part_number: \"BUS\", power: Some(Conduit(max_a: {headroom}))),"
	);
	let _ = writeln!(
		s,
		"(id: \"BRK\", name: \"breaker\", manufacturer: \"Synth\", manufacturer_part_number: \"BRK\", power: Some(Gate(rating_a: {}))),",
		LOADS_PER_BREAKER as u32 * DRAW_PER_LOAD + 5
	);
	let _ = writeln!(
		s,
		"(id: \"LOAD\", name: \"lamp\", manufacturer: \"Synth\", manufacturer_part_number: \"LOAD\", power: Some(Load(draw_a: {DRAW_PER_LOAD}))),"
	);
	s.push_str("],\n");

	s.push_str("modules: [\n");
	s.push_str("(label: \"PWR-01\", room: \"R1\", part: \"CELL\", serial: \"S-0\", made: 2150.0, condition: 1.0),\n");
	s.push_str("(label: \"BUS-01\", room: \"R1\", part: \"BUS\", serial: \"S-1\", made: 2150.0, condition: 1.0),\n");
	for b in 0..breakers {
		let _ = writeln!(
			s,
			"(label: \"BRK-{b:05}\", room: \"R1\", part: \"BRK\", serial: \"S-B{b}\", made: 2150.0, condition: 1.0),"
		);
	}
	for l in 0..loads {
		let _ = writeln!(
			s,
			"(label: \"LD-{l:05}\", room: \"R1\", part: \"LOAD\", serial: \"S-L{l}\", made: 2150.0, condition: 1.0),"
		);
	}
	s.push_str("],\n");

	s.push_str("connections: [\n");
	s.push_str(
		"(net: Power, from: (\"PWR-01\", \"out\"), to: (\"BUS-01\", \"in\"), run: \"TRUNK\"),\n",
	);
	for b in 0..breakers {
		let _ = writeln!(
			s,
			"(net: Power, from: (\"BUS-01\", \"out{b}\"), to: (\"BRK-{b:05}\", \"in\"), run: \"TRUNK\"),"
		);
	}
	for l in 0..loads {
		let _ = writeln!(
			s,
			"(net: Power, from: (\"BRK-{:05}\", \"out\"), to: (\"LD-{l:05}\", \"pwr\"), run: \"TRUNK\"),",
			l % breakers
		);
	}
	s.push_str("],\n)\n");
	s
}

// ---- instruments -------------------------------------------------------

/// The only wall-clock in the project, and it is outside the sim looking in.
#[allow(
	clippy::disallowed_methods,
	reason = "the universe has one clock; this measures it from outside and never enters it"
)]
fn timed<T>(f: impl FnOnce() -> T) -> (T, Duration) {
	let start = Instant::now();
	let out = f();
	(out, start.elapsed())
}

/// Resident set size in KiB. Linux only, and deliberately crude — it counts
/// what the allocator actually holds, which is the honest number when the
/// question is whether a million `String` labels fit.
fn rss_kb() -> u64 {
	std::fs::read_to_string("/proc/self/status")
		.ok()
		.and_then(|s| {
			s.lines()
				.find(|l| l.starts_with("VmRSS:"))
				.and_then(|l| l.split_whitespace().nth(1).and_then(|n| n.parse().ok()))
		})
		.unwrap_or(0)
}

fn arg(args: &mut impl Iterator<Item = String>, default: usize) -> usize {
	args.next().and_then(|s| s.parse().ok()).unwrap_or(default)
}

fn per_unit(d: Duration, n: usize) -> f64 {
	d.as_micros() as f64 / n.max(1) as f64
}

fn bytes_per(kib: u64, n: usize) -> f64 {
	(kib as f64 * 1024.0) / n.max(1) as f64
}

fn report(what: &str, d: Duration) {
	println!("  {what:<8} {:>10.1} ms", d.as_secs_f64() * 1000.0);
}
