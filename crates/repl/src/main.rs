#![allow(clippy::print_stdout, clippy::print_stderr)]
use sim::{
	Command, Condition, Event, EventKind, ModuleId, PowerRole, Symptom, Universe, World, tick,
};
use std::io::{self, BufRead, Write};
use std::ops::ControlFlow::{self, Break, Continue};
fn main() {
	let path = std::env::args()
		.nth(1)
		.unwrap_or_else(|| "fixtures/testudo.ron".to_string());

	let world = match World::from_fixture_file(&path) {
		Ok(w) => w,
		Err(e) => {
			eprintln!("deck.link failed: {e:?}");
			std::process::exit(1);
		}
	};
	let mut universe: Universe = Universe::new();
	let id = universe.add_world(world);

	println!("> deck.link ............ CONNECTED");
	println!(
		"> {} modules aboard. 'help' for verbs.",
		universe.world(id).map_or(0, |world| world.modules().len())
	);
	print_usage();

	let stdin = io::stdin();
	let mut queued: Vec<Command> = Vec::new();

	let mut history: Vec<Vec<Command>> = Vec::new();

	while let Some(world) = universe.world_mut(id) {
		print!("deck> ");
		let _ = io::stdout().flush();

		let Some(Ok(line)) = stdin.lock().lines().next() else {
			break;
		};
		let words = line.split_whitespace();

		if dispatch(&path, &mut queued, &mut history, words, world).is_break() {
			break;
		}
	}
}

fn dispatch(
	path: &str,
	queued: &mut Vec<Command>,
	history: &mut Vec<Vec<Command>>,
	mut words: std::str::SplitWhitespace<'_>,
	world: &mut World,
) -> ControlFlow<(), ()> {
	match (words.next(), words.next()) {
		(Some("quit" | "q" | "exit"), _) => Break(()),

		(Some("list"), _) => {
			for (id, m) in world.modules() {
				println!(
					"  [{:>2}] {:<8} {:<12}",
					id.index(),
					m.label,
					world.part(*id).map_or(UNLISTED, |p| p.name.as_str())
				);
			}
			Continue(())
		}

		(Some("inspect"), Some(label)) => {
			print_inspect(world, label);
			Continue(())
		}
		(Some("status"), Some(label)) => {
			print_status(world, label);
			Continue(())
		}

		(Some("source"), Some(onoff)) => {
			if let (Some(label), "on" | "off") = (words.next(), onoff) {
				queue_source(world, queued, label, onoff == "on");
			} else {
				println!("  usage: source on|off <LABEL>");
			}
			Continue(())
		}

		(Some("breaker"), Some(openclose)) => {
			if let (Some(label), "open" | "close") = (words.next(), openclose) {
				queue_breaker(world, queued, label, openclose == "close");
			} else {
				println!("  usage: breaker open|close <LABEL>");
			}
			Continue(())
		}

		(Some("tick"), n) => {
			let n: u64 = n.and_then(|s| s.parse().ok()).unwrap_or(1);
			for _ in 0..n {
				history.push(queued.clone());
				let events = tick(world, &*queued);
				queued.clear();
				for ev in &events {
					print_event(world, ev);
				}
			}
			Continue(())
		}
		(Some("condition"), Some(label)) => {
			if let Some(value) = words.next() {
				queue_condition(world, queued, label, value);
			} else {
				println!(" usage: condition <LABEL> <0.0-1.0>");
			}
			Continue(())
		}
		(Some("save"), Some(file)) => {
			save_history(&*history, file);
			Continue(())
		}
		(Some("replay"), Some(file)) => {
			replay_history(path, file);
			Continue(())
		}

		(Some("log"), n) => {
			let n: usize = n.and_then(|s| s.parse().ok()).unwrap_or(10);
			for ev in world.log().iter().rev().take(n).rev() {
				print_event(world, ev);
			}
			Continue(())
		}
		(Some("scan"), _) => {
			print_scan(world);
			Continue(())
		}

		(Some("help" | "?"), _) => {
			print_usage();
			Continue(())
		}

		(Some(cmd), _) => {
			println!("  unknown verb '{cmd}'.");
			print_usage();
			Continue(())
		}
		(None, _) => Continue(()),
	}
}

/// The deck's grammar. Grows a line per phase; keep it the only place the
/// verbs are spelled out, so it cannot drift from what the parser accepts.
/// What the deck prints where a catalogue entry should be. Unreachable while
/// the resolver mints every `PartId` — but a display that says "I do not know"
/// beats one that invents a name.
const UNLISTED: &str = "(unlisted)";

fn print_usage() {
	for (verb, what) in [
		("list", "every module aboard"),
		("inspect <LABEL>", "meta: maker, part, serial, made"),
		("status <LABEL>", "power state, draw, actuator position"),
		("scan", "scan the ship"),
		("source on|off <LABEL>", "queue a source command"),
		("condition <LABEL> <0.0-1.0>", "set a module's condition"),
		("breaker open|close <LABEL>", "queue a breaker command"),
		("tick [n]", "submit the queue, run n ticks (default 1)"),
		("log [n]", "last n events, causes as ← #id (default 10)"),
		("help", "this"),
		("quit", "deck.halt"),
	] {
		println!("  {verb:<28}{what}");
	}
}

/// The paperwork on one module: what it is, who made it, and how worn. Says
/// nothing about how it is behaving — that is `status`, and keeping the two
/// apart is what stops the deck naming a cause.
fn print_inspect(world: &World, label: &str) {
	let Some((id, m)) = world.modules().iter().find(|(_, m)| m.label == label) else {
		println!("  no module '{label}' aboard.");
		return;
	};
	let cond = world.condition_of(*id).map(Condition::get);
	match world.part(*id) {
		Some(part) => {
			println!("  {} — {}", m.label, part.name);
			println!(
				"  maker  {:<10} part {}",
				part.manufacturer, part.manufacturer_part_number
			);
		}
		None => println!("  {} — {UNLISTED}", m.label),
	}
	println!("  serial {}", m.serial);
	println!("  made   {:.2}    condition {:.2?}", m.made, cond);
}

/// Per-network state for one module. Power only, until phase 3 gives the
/// valves something to say.
fn print_status(world: &World, label: &str) {
	let Some(id) = world.find_label(label) else {
		println!("  no module '{label}' aboard.");
		return;
	};
	let Some(m) = world.modules().get(&id) else {
		println!("  no module '{label}' aboard.");
		return;
	};
	println!(
		"  {} — {}",
		m.label,
		world.part(id).map_or(UNLISTED, |p| p.name.as_str())
	);
	println!(
		"  power   {:<10} draw {}A",
		match world.is_energised(id) {
			Some(true) => "ENERGISED",
			Some(false) => "dark",
			None => "n/a",
		},
		world.power_role(id).map_or(0, PowerRole::draw_a)
	);
	if let Some(closed) = world.breaker_closed(id) {
		println!("  breaker {}", if closed { "closed" } else { "OPEN" });
	}
	if let Some(online) = world.source_online(id) {
		println!("  source  {}", if online { "ONLINE" } else { "offline" });
		if let Some(charge) = world.charge_of(id) {
			println!("  charge  {charge} amp-ticks");
		}
	}
}

fn queue_source(world: &World, queued: &mut Vec<Command>, label: &str, online: bool) {
	match world.find_label(label) {
		Some(id) => {
			queued.push(Command::SetSource { id, online });
			println!("  queued. (commands land on 'tick')");
		}
		None => println!("  no module '{label}' aboard."),
	}
}

fn queue_breaker(world: &World, queued: &mut Vec<Command>, label: &str, closed: bool) {
	match world.find_label(label) {
		Some(id) => {
			queued.push(Command::SetBreaker { id, closed });
			println!("  queued. (commands land on 'tick')");
		}
		None => println!("  no module '{label}' aboard."),
	}
}
fn queue_condition(world: &World, queued: &mut Vec<Command>, label: &str, condition: &str) {
	match world.find_label(label) {
		Some(id) => {
			let Ok(value) = condition.parse::<f32>() else {
				println!("Value {condition} is not a valid condition");
				return;
			};
			let Ok(cond) = Condition::new(value) else {
				println!("Value {value:.2} is outside of bounds, must be within 0.0-1.0");
				return;
			};
			queued.push(Command::SetCondition {
				id,
				new_condition: cond,
			});
			println!("  queued. (commands land on 'tick')");
		}
		None => println!("  no module '{label}' aboard."),
	}
}

fn print_event(world: &World, ev: &Event) {
	let name = |id: &ModuleId| {
		world
			.modules()
			.get(id)
			.map_or_else(|| "???".to_string(), |m| m.label.clone())
	};
	let cause = ev.cause.map_or(String::new(), |c| format!("  ← #{}", c.0));
	let what = match &ev.kind {
		EventKind::WorldLoaded => "world loaded".to_string(),
		EventKind::SourceSet { id, online } => format!(
			"{} {}",
			name(id),
			if *online { "ONLINE" } else { "offline" }
		),
		EventKind::BreakerSet { id, closed } => {
			format!("{} {}", name(id), if *closed { "closed" } else { "OPENED" })
		}
		EventKind::PowerChanged { id, energised } => format!(
			"{} {}",
			name(id),
			if *energised { "energised" } else { "DARK" }
		),
		EventKind::BreakerTripped {
			id,
			load_a,
			rating_a,
			degraded_rating_a,
		} => format!(
			"{} TRIPPED — {}A on a {}A breaker - Effectively {}A",
			name(id),
			load_a,
			rating_a,
			degraded_rating_a
		),
		EventKind::CapacityExceeded {
			id,
			load_a,
			rating_a,
			degraded_rating_a,
		} => format!(
			"{} OVERLOADED - {}A on a bus rated for {}A - Effectively {}A",
			name(id),
			load_a,
			rating_a,
			degraded_rating_a
		),
		EventKind::SourceDepleted { id } => format!("{} DEPLETED — flat", name(id)),
		EventKind::CommandRejected { reason } => format!("rejected: {reason:?}"),
		EventKind::ConditionChanged { id, from, to } => {
			format!("{} condition changed from {from:.2} to {to:.2}", name(id))
		}
	};
	println!("  #{:<4} t{:<4} {}{}", ev.id.0, ev.at_tick, what, cause);
}
fn save_history(history: &[Vec<Command>], file: &str) {
	let cfg = ron::ser::PrettyConfig::default();
	match ron::ser::to_string_pretty(&history, cfg) {
		Ok(text) => match std::fs::write(file, text) {
			Ok(()) => println!("  {} batches → {file}", history.len()),
			Err(e) => println!("  cannot write {file}: {e}"),
		},
		Err(e) => println!("  cannot serialise: {e}"),
	}
}

fn replay_history(fixture: &str, file: &str) {
	let (Ok(text), Ok(fixture_text)) = (
		std::fs::read_to_string(file),
		std::fs::read_to_string(fixture),
	) else {
		println!("  cannot read {file} or {fixture}.");
		return;
	};
	let script: Vec<Vec<Command>> = match ron::from_str(&text) {
		Ok(s) => s,
		Err(e) => {
			println!("  cannot parse {file}: {e}");
			return;
		}
	};
	match sim::replay(&fixture_text, &script) {
		Ok(w) => println!(
			"  refolded {} batches → tick {}, {} events",
			script.len(),
			w.tick(),
			w.log().len()
		),
		Err(e) => println!("  replay failed: {e:?}"),
	}
}
fn print_scan(world: &World) {
	for (id, m) in world.modules() {
		println!(
			"  [{:>2}] {:<8} {:<12} Status: {}",
			id.index(),
			m.label,
			world.part(*id).map_or(UNLISTED, |p| p.name.as_str()),
			match world.symptom_of(*id) {
				Some(Symptom::Dark) => {
					"Dark"
				}
				Some(Symptom::Nominal) => {
					"Nominal"
				}
				Some(Symptom::Starved) => {
					"Starved"
				}
				None => {
					"N/A"
				}
			}
		);
	}
}
