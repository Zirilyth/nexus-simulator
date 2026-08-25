use std::collections::BTreeMap;

use crate::systems::power::PowerNet;
use crate::types::catalogue::{Part, PartId};
use crate::types::condition::Condition;
use crate::types::ids::NetworkKind;
use crate::types::modules::{ModuleDef, ModuleId, ModuleMeta, PortName};
use crate::types::role::PowerRole;
use crate::types::world::{Connection, LoadError};
use crate::{EventKind, World};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipFixture {
	pub class: String,
	pub hull_no: String,
	pub commissioned: f64,
	pub seed: u64,
	pub parts: Vec<PartDef>,
	pub rooms: Vec<RoomDef>,
	pub runs: Vec<RunDef>,
	pub modules: Vec<ModuleDef>,
	pub connections: Vec<ConnDef>,
}
#[derive(Debug)]
pub struct ResolvedShip {
	pub seed: u64,
	pub modules: Vec<(ModuleId, ModuleMeta, Condition)>,
	pub connections: Vec<Connection>,
	pub parts: Vec<Part>,
	pub port_names: Vec<String>,
}

impl TryFrom<ShipFixture> for ResolvedShip {
	type Error = LoadError;
	fn try_from(fx: ShipFixture) -> Result<Self, LoadError> {
		let mut ids: BTreeMap<String, ModuleId> = BTreeMap::new();
		let mut modules = Vec::new();

		let mut part_ids: BTreeMap<String, PartId> = BTreeMap::new();
		let mut parts: Vec<Part> = Vec::new();
		for (i, p) in fx.parts.iter().enumerate() {
			let pid = PartId(u32::try_from(i).expect("more than 4 billion parts in one catalogue"));
			part_ids.insert(p.id.clone(), pid);
			parts.push(Part {
				name: p.name.clone(),
				manufacturer: p.manufacturer.clone(),
				manufacturer_part_number: p.manufacturer_part_number.clone(),
				power: p.power,
			});
		}

		for (i, m) in fx.modules.iter().enumerate() {
			let id = ModuleId(u32::try_from(i).expect("more than 4 billion modules on one hull"));
			ids.insert(m.label.clone(), id);
			modules.push((
				id,
				ModuleMeta {
					label: m.label.clone(),
					part: *part_ids
						.get(&m.part)
						.ok_or_else(|| LoadError::UnknownPart(m.part.clone()))?,
					serial: m.serial.clone(),
					made: m.made,
				},
				m.condition,
			));
		}

		let mut port_names: Vec<String> = Vec::new();
		let mut seen: BTreeMap<String, PortName> = BTreeMap::new();

		let mut intern = |text: &str| -> PortName {
			if let Some(p) = seen.get(text) {
				return *p;
			}
			let p = PortName(
				u32::try_from(port_names.len()).expect("more than 4 billion distinct port names"),
			);
			port_names.push(text.to_string());
			seen.insert(text.to_string(), p);
			p
		};

		// pass 2: resolution — YOUR connections loop, verbatim
		let mut connections = Vec::new();

		for c in &fx.connections {
			let resolve = |label: &str| {
				ids.get(label)
					.copied()
					.ok_or_else(|| LoadError::UnknownLabel(label.into()))
			};
			let from = (resolve(&c.from.0)?, intern(&c.from.1));
			let to = (resolve(&c.to.0)?, intern(&c.to.1));
			connections.push(Connection {
				net: c.net,
				from,
				to,
				run: c.run.clone(),
			});
		}

		Ok(ResolvedShip {
			seed: fx.seed,
			modules,
			connections,
			parts,
			port_names,
		})
		//  ^ the tail expression. no `return`. this is the value.
	}
}

impl From<ResolvedShip> for World {
	fn from(ship: ResolvedShip) -> Self {
		let mut modules = BTreeMap::new();
		let mut condition = BTreeMap::new();
		for (id, meta, cond) in ship.modules {
			modules.insert(id, meta);
			condition.insert(id, cond);
		}
		let power = PowerNet::from_modules(&ship.parts, &modules);

		let mut world = World {
			modules,
			condition,
			power,
			parts: ship.parts,
			connections: ship.connections, // …then the original moves. no waste, no assignment
			..World::new(ship.seed)        // tick, rng, log from the one blessed constructor
		};
		// event #0, always. paracausal: the ship exists because I said so.
		world.emit(EventKind::WorldLoaded, None);
		world
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDef {
	pub id: String,
	pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDef {
	pub id: String,
	pub cap: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnDef {
	pub net: NetworkKind,
	pub from: (String, String),
	pub to: (String, String),
	pub run: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct PartDef {
	pub id: String,
	pub name: String,
	pub manufacturer: String,
	pub manufacturer_part_number: String,
	pub power: Option<PowerRole>,
}
