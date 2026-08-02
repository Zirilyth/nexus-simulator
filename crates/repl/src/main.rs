use sim::{ModuleKind, World};
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

	loop {
		print!("deck> ");
		io::stdout().flush().unwrap();

		let Some(Ok(line)) = stdin.lock().lines().next() else {
			break;
		};
		let mut words = line.split_whitespace();

		match (words.next(), words.next()) {
			(Some("quit"), _) | (Some("q"), _) => break,

			(Some("list"), _) => {
				for (id, m) in &world.modules {
					let cond = world.condition.get(id).copied().unwrap_or(0.0);
					println!(
						"  [{:>2}] {:<8} {:<12} cond {:.2}",
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
						let cond = world.condition.get(id).copied().unwrap_or(0.0);
						println!("  {} — {}", m.label, kind_name(&m.kind));
						println!("  maker  {:<10} part {}", m.maker, m.part);
						println!("  serial {}", m.serial);
						println!("  made   {:.2}    condition {:.2}", m.made, cond);
					}
					None => println!("  no module '{label}' aboard."),
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
