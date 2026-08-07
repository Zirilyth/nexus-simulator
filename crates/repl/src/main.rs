#![allow(clippy::print_stdout, clippy::print_stderr)]
use sim::{Command, Condition, Event, EventKind, ModuleId, ModuleKind, World, tick};
use std::io::{self, BufRead, Write};
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

	println!("> deck.link ............ CONNECTED");
	println!(
		"> {} modules aboard. 'help' for verbs.",
		world.modules().len()
	);
	print_usage();

	let stdin = io::stdin();
	let mut world = world; // she mutates now
	let mut queued: Vec<Command> = Vec::new();

	let mut history: Vec<Vec<Command>> = Vec::new();

	loop {
		print!("deck> ");
		let _ = io::stdout().flush();

		let Some(Ok(line)) = stdin.lock().lines().next() else {
			break;
		};
		let mut words = line.split_whitespace();

		match (words.next(), words.next()) {
			(Some("quit" | "q" | "exit"), _) => break,

			(Some("list"), _) => {
				for (id, m) in world.modules() {
					let cond = world.condition_of(*id);
					println!(
						"  [{:>2}] {:<8} {:<12} cond {:.2?}",
						id.0,
						m.label,
						kind_name(&m.kind),
						cond
					);
				}
			}

			(Some("inspect"), Some(label)) => {
				match world.modules().iter().find(|(_, m)| m.label == label) {
					Some((id, m)) => {
						let cond = world.condition_of(*id).map(Condition::get);
						println!("  {} — {}", m.label, kind_name(&m.kind));
						println!("  maker  {:<10} part {}", m.maker, m.part);
						println!("  serial {}", m.serial);
						println!("  made   {:.2}    condition {:.2?}", m.made, cond);
					}
					None => println!("  no module '{label}' aboard."),
				}
			}
			(Some("status"), Some(label)) => print_status(&world, label),

			(Some("source"), Some(onoff)) => match (words.next(), onoff) {
				(Some(label), "on" | "off") => {
					queue_source(&world, &mut queued, label, onoff == "on");
				}
				_ => println!("  usage: source on|off <LABEL>"),
			},

			(Some("breaker"), Some(openclose)) => match (words.next(), openclose) {
				(Some(label), "open" | "close") => {
					queue_breaker(&world, &mut queued, label, openclose == "close");
				}
				_ => println!("  usage: breaker open|close <LABEL>"),
			},

			(Some("tick"), n) => {
				let n: u64 = n.and_then(|s| s.parse().ok()).unwrap_or(1);
				for _ in 0..n {
					history.push(queued.clone());
					let events = tick(&mut world, &queued);
					queued.clear();
					for ev in &events {
						print_event(&world, ev);
					}
				}
			}
			(Some("condition"), Some(label)) => match words.next() {
				Some(value) => queue_condition(&world, &mut queued, label, value),
				None => println!(" usage: condition <LABEL> <0.0-1.0>"),
			},
			(Some("save"), Some(file)) => save_history(&history, file),
			(Some("replay"), Some(file)) => replay_history(&path, file),

			(Some("log"), n) => {
				let n: usize = n.and_then(|s| s.parse().ok()).unwrap_or(10);
				for ev in world.log().iter().rev().take(n).rev() {
					print_event(&world, ev);
				}
			}

			(Some("help" | "?"), _) => print_usage(),

			(Some(cmd), _) => {
				println!("  unknown verb '{cmd}'.");
				print_usage();
			}
			(None, _) => {}
		}
	}
}

fn kind_name(k: &ModuleKind) -> &'static str {
	match k {
		ModuleKind::BatteryBank { .. } => "battery",
		ModuleKind::Bus { .. } => "bus",
		ModuleKind::Breaker { .. } => "breaker",
		ModuleKind::Scrubber => "scrubber",
		ModuleKind::Heater => "heater",
		ModuleKind::Pump => "pump",
		ModuleKind::Sensor => "sensor",
		ModuleKind::Lights => "lights",
		ModuleKind::Console => "console",
		ModuleKind::Valve { .. } => "valve",
	}
}
/// The deck's grammar. Grows a line per phase; keep it the only place the
/// verbs are spelled out, so it cannot drift from what the parser accepts.
fn print_usage() {
	for (verb, what) in [
		("list", "every module aboard"),
		("inspect <LABEL>", "meta: maker, part, serial, made"),
		("status <LABEL>", "power state, draw, actuator position"),
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

/// Per-network state for one module. Power only, until phase 3 gives the
/// valves something to say.
fn print_status(world: &World, label: &str) {
	let Some(id) = world.find_label(label) else {
		println!("  no module '{label}' aboard.");
		return;
	};
	let m = &world.modules()[&id];
	println!("  {} — {}", m.label, kind_name(&m.kind));
	println!(
		"  power   {:<10} draw {}A",
		match world.is_energised(id) {
			Some(true) => "ENERGISED",
			Some(false) => "dark",
			None => "n/a",
		},
		m.draw_a
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
			.map_or("???".to_string(), |m| m.label.clone())
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
