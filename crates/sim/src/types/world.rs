use crate::types::events::Event;
use crate::types::fixture::ShipFixture;
use crate::types::ids::{ModuleId, NetworkKind, PortName};
use crate::types::meta::{ModuleKind, ModuleMeta};
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
	pub net: NetworkKind,
	pub from: (ModuleId, PortName),
	pub to: (ModuleId, PortName),
	pub run: String,
}

pub struct World {
	pub tick: u64,
	pub rng: ChaCha8Rng,
	pub modules: BTreeMap<ModuleId, ModuleMeta>,
	pub condition: BTreeMap<ModuleId, f32>,
	pub connections: Vec<Connection>,
	pub as_built: Vec<Connection>,
	pub log: Vec<Event>,
}

#[derive(Debug)]
pub enum LoadError {
	Io(std::io::Error),
	Parse(ron::error::SpannedError),
	UnknownLabel(String),
}

impl World {
	pub fn from_fixture_str(text: &str) -> Result<Self, LoadError> {
		let fx: ShipFixture = ron::from_str(text).map_err(LoadError::Parse)?;
		let mut world = World::new(fx.seed);

		let mut ids: BTreeMap<String, ModuleId> = BTreeMap::new();

		for (i, m) in fx.modules.iter().enumerate() {
			let id = ModuleId(i as u32);
			ids.insert(m.label.clone(), id);
			world.modules.insert(
				id,
				ModuleMeta {
					kind: m.kind.clone(),
					label: m.label.clone(),
					maker: m.maker.clone(),
					part: m.part.clone(),
					serial: m.serial.clone(),
					made: m.made,
				},
			);
			world.condition.insert(id, m.condition);
		} // ← modules loop CLOSES here. all labels now known.

		for c in &fx.connections {
			let resolve = |label: &str| {
				ids.get(label)
					.copied()
					.ok_or_else(|| LoadError::UnknownLabel(label.into()))
			};
			world.connections.push(Connection {
				net: c.net,
				from: (resolve(&c.from.0)?, PortName(c.from.1.clone())),
				to: (resolve(&c.to.0)?, PortName(c.to.1.clone())),
				run: c.run.clone(),
			});
		}

		world.as_built = world.connections.clone();
		Ok(world)
	}

	pub fn from_fixture_file(path: &str) -> Result<Self, LoadError> {
		let text = std::fs::read_to_string(path).map_err(LoadError::Io)?;
		Self::from_fixture_str(&text)
	}

	pub fn new(seed: u64) -> Self {
		World {
			tick: 0,
			rng: ChaCha8Rng::seed_from_u64(seed),
			modules: BTreeMap::new(),
			condition: BTreeMap::new(),
			connections: Vec::new(),
			as_built: Vec::new(),
			log: Vec::new(),
		}
	}
}

#[test]
fn testudo_comes_aboard() {
	let w = World::from_fixture_file(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/../../fixtures/testudo.ron"
	))
	.expect("testudo should load");

	assert_eq!(w.modules.len(), 14);
	assert_eq!(w.connections.len(), 14);
	assert_eq!(w.as_built, w.connections); // true today. wait a decade.

	let brk = w.modules.values().find(|m| m.label == "BRK-01").unwrap();
	assert!(matches!(brk.kind, ModuleKind::Breaker { rating_a: 25 }));
}
