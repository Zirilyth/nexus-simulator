#![allow(clippy::print_stdout, clippy::print_stderr)]
use sim::{Command, Event, EventKind, ModuleKind, World, tick};
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
		"> {} modules aboard. 'list', 'inspect <LABEL>', 'quit'",
		world.modules.len()
	);

	let stdin = io::stdin();
	let mut world = world; // she mutates now
	let mut queued: Vec<Command> = Vec::new();

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
				for (id, m) in &world.modules {
					let cond = world.condition.get(id);
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
				match world.modules.iter().find(|(_, m)| m.label == label) {
					Some((id, m)) => {
						let cond = world.condition.get(id).copied();
						println!("  {} — {}", m.label, kind_name(&m.kind));
						println!("  maker  {:<10} part {}", m.maker, m.part);
						println!("  serial {}", m.serial);
						println!("  made   {:.2}    condition {:.2?}", m.made, cond);
					}
					None => println!("  no module '{label}' aboard."),
				}
			}
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
					let events = tick(&mut world, &queued);
					queued.clear();
					for ev in &events {
						print_event(&world, ev);
					}
				}
			}

			(Some("log"), n) => {
				let n: usize = n.and_then(|s| s.parse().ok()).unwrap_or(10);
				for ev in world.log.iter().rev().take(n).rev() {
					print_event(&world, ev);
				}
			}

			(Some(cmd), _) => println!("  unknown verb '{cmd}'."),
			(None, _) => {}
		}
	}
}

fn kind_name(k: &ModuleKind) -> &'static str {
	match k {
		ModuleKind::BatteryBank => "battery",
		ModuleKind::Bus => "bus",
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

fn print_event(world: &World, ev: &Event) {
	let name = |id: &sim::types::modules::ModuleId| {
		world
			.modules
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
		} => format!(
			"{} TRIPPED — {}A on a {}A breaker",
			name(id),
			load_a,
			rating_a
		),
		EventKind::CommandRejected { reason } => format!("rejected: {reason:?}"),
	};
	println!("  #{:<4} t{:<4} {}{}", ev.id.0, ev.at_tick, what, cause);
}
