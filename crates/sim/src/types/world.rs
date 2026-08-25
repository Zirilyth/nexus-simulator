use crate::systems::power::{self, PowerNet, sagging_suppliers};
use crate::types::catalogue::{Part, Run};
use crate::types::condition::Condition;
use crate::types::events::Event;
use crate::types::fixture::{ResolvedShip, ShipFixture};
use crate::types::ids::{NetworkKind, RunId};
use crate::types::modules::{ModuleId, ModuleMeta, PortName};
use crate::types::role::PowerRole;
use crate::types::symptom::Symptom;
use crate::{EventId, EventKind};
use rand_chacha::ChaCha8Rng;
use rand_chacha::rand_core::SeedableRng;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
	pub net: NetworkKind,
	pub from: (ModuleId, PortName),
	pub to: (ModuleId, PortName),
	pub run: RunId,
}
#[derive(Debug, PartialEq)]
pub struct World {
	pub(crate) tick: u64,
	pub(crate) next_event: u64,
	pub(crate) rng: ChaCha8Rng,
	pub(crate) modules: BTreeMap<ModuleId, ModuleMeta>,
	pub(crate) condition: BTreeMap<ModuleId, Condition>,
	pub(crate) connections: Vec<Connection>,
	pub(crate) power: PowerNet,
	pub(crate) log: Vec<Event>,
	pub(crate) parts: Vec<Part>,
	pub(crate) port_names: Vec<String>,
	pub(crate) runs: Vec<Run>,
}

#[derive(Debug)]
pub enum LoadError {
	Io(std::io::Error),
	Parse(ron::error::SpannedError),
	UnknownLabel(String),
	UnknownPart(String),
	UnknownRun(String),
}

impl World {
	#[must_use]
	pub fn log(&self) -> &[Event] {
		&self.log
	}

	#[must_use]
	pub fn tick(&self) -> u64 {
		self.tick
	}

	#[must_use]
	pub fn modules(&self) -> &BTreeMap<ModuleId, ModuleMeta> {
		&self.modules
	}

	#[must_use]
	pub fn condition_of(&self, id: ModuleId) -> Option<Condition> {
		self.condition.get(&id).copied()
	}

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
	pub(crate) fn new(seed: u64) -> Self {
		World {
			tick: 0,
			next_event: 0,
			rng: ChaCha8Rng::seed_from_u64(seed),
			modules: BTreeMap::new(),
			condition: BTreeMap::new(),
			connections: Vec::new(),
			power: PowerNet::default(),
			log: Vec::new(),
			parts: Vec::new(),
			port_names: Vec::new(),
			runs: Vec::new(),
		}
	}
	#[must_use]
	pub fn find_label(&self, label: &str) -> Option<ModuleId> {
		self.modules
			.iter()
			.find(|(_, m)| m.label == label)
			.map(|(id, _)| *id)
	}

	/// `None` = this module has no opinion about electricity (a valve, so far).
	#[must_use]
	pub fn is_energised(&self, id: ModuleId) -> Option<bool> {
		self.power.states.get(&id).map(|s| s.energised)
	}

	/// `None` = not a breaker.
	#[must_use]
	pub fn breaker_closed(&self, id: ModuleId) -> Option<bool> {
		self.power.breakers.get(&id).map(|b| b.closed)
	}

	/// `None` = not a source.
	#[must_use]
	pub fn source_online(&self, id: ModuleId) -> Option<bool> {
		self.power.sources.get(&id).map(|s| s.online)
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
	#[must_use]
	pub fn port_name(&self, p: PortName) -> &str {
		&self.port_names[p.0 as usize]
	}
	#[must_use]
	pub fn symptom_of(&self, id: ModuleId) -> Option<Symptom> {
		if self.is_energised(id)? {
			if sagging_suppliers(self).contains(&id) {
				Some(Symptom::Starved)
			} else {
				Some(Symptom::Nominal)
			}
		} else {
			Some(Symptom::Dark)
		}
	}

	#[must_use]
	pub fn charge_of(&self, id: ModuleId) -> Option<u64> {
		self.power.sources.get(&id).map(|s| s.charge_at(self.tick))
	}
	#[must_use]
	pub fn part(&self, id: ModuleId) -> &Part {
		let part_id = self.modules[&id].part;
		&self.parts[part_id.0 as usize]
	}

	#[must_use]
	pub fn run(&self, id: RunId) -> &Run {
		&self.runs[id.0 as usize]
	}

	#[must_use]
	pub fn power_role(&self, id: ModuleId) -> Option<PowerRole> {
		self.part(id).power
	}

	#[must_use]
	pub fn next_event_at(&self) -> Option<u64> {
		power::next_event_at(&self.power)
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

	let brk = w.find_label("BRK-01").expect("BRK-01 is aboard");
	assert!(matches!(
		w.power_role(brk),
		Some(PowerRole::Gate { rating_a: 25 })
	));
}

#[test]
fn next_event_at_sees_everything_scheduled() {
	use crate::types::events::Command;

	let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/testudo.ron");
	let load = || World::from_fixture_file(fixture).expect("testudo should load");
	let power_up = |w: &World| {
		vec![
			Command::SetSource {
				id: w.find_label("PWR-01").expect("PWR-01 is aboard"),
				online: true,
			},
			Command::SetBreaker {
				id: w.find_label("BRK-01").expect("BRK-01 is aboard"),
				closed: true,
			},
			Command::SetBreaker {
				id: w.find_label("BRK-02").expect("BRK-02 is aboard"),
				closed: true,
			},
		]
	};

	// asleep. nothing is scheduled on a dead ship, and a fleet of hulls all
	// answering None is a fleet that costs one comparison each to skip.
	assert_eq!(load().next_event_at(), None, "a dark ship waits forever");

	// powered: the battery's death is already solved, not counted down to.
	// 144_000 amp-ticks at 29A — the same number the_battery_dies pins.
	let mut w = load();
	let script = power_up(&w);
	let _ = crate::tick(&mut w, &script);
	assert_eq!(
		w.next_event_at(),
		Some(4965),
		"the anchor knows when she dies"
	);

	// a scheduled trip is sooner, so it wins. this is the half of the chain
	// that reads pending_trips — drop it and the assertion above still passes.
	let mut w = load();
	let mut script = vec![Command::SetCondition {
		id: w.find_label("BRK-01").expect("BRK-01 is aboard"),
		new_condition: Condition::new(0.5).expect("0.5 is a condition"),
	}];
	script.extend(power_up(&w));
	let _ = crate::tick(&mut w, &script);
	assert_eq!(
		w.next_event_at(),
		Some(1),
		"a breaker tripped this tick opens the next one"
	);
}
