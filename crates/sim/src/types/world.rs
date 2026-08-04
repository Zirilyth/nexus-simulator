use crate::systems::power::PowerNet;
use crate::types::condition::Condition;
use crate::types::events::Event;
use crate::types::fixture::{ResolvedShip, ShipFixture};
use crate::types::ids::NetworkKind;
use crate::types::modules::{ModuleId, ModuleMeta, PortName};
use crate::{EventId, EventKind};
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
#[derive(Debug)]
pub struct World {
	pub tick: u64,
	pub next_event: u64,
	pub rng: ChaCha8Rng,
	pub modules: BTreeMap<ModuleId, ModuleMeta>,
	pub condition: BTreeMap<ModuleId, Condition>,
	pub connections: Vec<Connection>,
	pub as_built: Vec<Connection>,
	pub power: PowerNet,
	pub log: Vec<Event>,
}

#[derive(Debug)]
pub enum LoadError {
	Io(std::io::Error),
	Parse(ron::error::SpannedError),
	UnknownLabel(String),
}

impl World {
	/// Load a ship from RON fixture text.
	///
	/// # Errors
	/// - [`LoadError::Parse`] — malformed RON (span included)
	/// - [`LoadError::UnknownLabel`] — a connection references a module not aboard
	pub fn from_fixture_str(text: &str) -> Result<Self, LoadError> {
		let fx: ShipFixture = ron::from_str(text).map_err(LoadError::Parse)?;
		let resolved = ResolvedShip::try_from(fx)?;
		Ok(World::from(resolved))
	}

	/// Load a ship from RON fixture text.
	///
	/// # Errors
	/// - [`LoadError::Parse`] — malformed RON (span included)
	/// - [`LoadError::UnknownLabel`] — a connection references a module not aboard
	pub fn from_fixture_file(path: &str) -> Result<Self, LoadError> {
		let text = std::fs::read_to_string(path).map_err(LoadError::Io)?;
		Self::from_fixture_str(&text)
	}

	#[must_use]
	pub fn new(seed: u64) -> Self {
		World {
			tick: 0,
			next_event: 0,
			rng: ChaCha8Rng::seed_from_u64(seed),
			modules: BTreeMap::new(),
			condition: BTreeMap::new(),
			connections: Vec::new(),
			as_built: Vec::new(),
			power: PowerNet::default(),
			log: Vec::new(),
		}
	}
	#[must_use]
	pub fn find_label(&self, label: &str) -> Option<ModuleId> {
		self.modules
			.iter()
			.find(|(_, m)| m.label == label)
			.map(|(id, _)| *id)
	}

	pub(crate) fn emit(&mut self, kind: EventKind, cause: Option<EventId>) -> EventId {
		let id = EventId(self.next_event);
		self.next_event += 1;
		self.log.push(Event {
			id,
			at_tick: self.tick,
			cause,
			kind,
		});
		id
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
	assert!(matches!(
		brk.kind,
		crate::ModuleKind::Breaker { rating_a: 25 }
	));
}
