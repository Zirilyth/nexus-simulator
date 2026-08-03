use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::types::condition::Condition;
use crate::types::ids::NetworkKind;
use crate::types::meta::ModuleKind;
use crate::types::world::LoadError;
use crate::{Connection, ModuleId, ModuleMeta, PortName, World};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipFixture {
	pub class: String,
	pub hull_no: String,
	pub commissioned: f64,
	pub seed: u64,
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
}

impl TryFrom<ShipFixture> for ResolvedShip {
	type Error = LoadError;
	fn try_from(fx: ShipFixture) -> Result<Self, LoadError> {
		let mut ids: BTreeMap<String, ModuleId> = BTreeMap::new();
		let mut modules = Vec::new();
		for (i, m) in fx.modules.iter().enumerate() {
			let id = ModuleId(u32::try_from(i).expect("more than 4 billion modules on one hull"));
			ids.insert(m.label.clone(), id);
			modules.push((
				id,
				ModuleMeta {
					kind: m.kind.clone(),
					label: m.label.clone(),
					maker: m.maker.clone(),
					part: m.part.clone(),
					serial: m.serial.clone(),
					made: m.made,
				},
				m.condition,
			));
		}

		// pass 2: resolution — YOUR connections loop, verbatim
		let mut connections = Vec::new();
		for c in &fx.connections {
			let resolve = |label: &str| {
				ids.get(label)
					.copied()
					.ok_or_else(|| LoadError::UnknownLabel(label.into()))
			};
			connections.push(Connection {
				net: c.net,
				from: (resolve(&c.from.0)?, PortName(c.from.1.clone())),
				to: (resolve(&c.to.0)?, PortName(c.to.1.clone())),
				run: c.run.clone(),
			});
		}

		Ok(ResolvedShip {
			seed: fx.seed,
			modules,
			connections,
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
		World {
			modules,
			condition,
			as_built: ship.connections.clone(), // evaluated first (source order)…
			connections: ship.connections,      // …then the original moves. no waste, no assignment
			..World::new(ship.seed)             // tick, rng, log from the one blessed constructor
		}
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
pub struct ModuleDef {
	pub label: String,
	pub kind: ModuleKind,
	pub room: String,
	pub maker: String,
	pub part: String,
	pub serial: String,
	pub made: f64,
	pub condition: Condition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnDef {
	pub net: NetworkKind,
	pub from: (String, String),
	pub to: (String, String),
	pub run: String,
}
