use std::collections::BTreeMap;

use crate::systems::power::PowerNet;
use crate::types::catalogue::{Part, PartId};
use crate::types::condition::Condition;
use crate::types::ids::{NetworkKind, RunId};
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
		let mut modules = Vec::new();

		let mut parts: Vec<Part> = Vec::new();

		let part_ids = index_by(&fx.parts, |p| &p.id, "parts in one catalogue")?;
		let run_ids = index_by(&fx.runs, |r| &r.id, "runs on one hull")?;
		let module_ids = index_by(&fx.modules, |m| &m.label, "modules on one hull")?;

		for p in fx.parts {
			parts.push(Part {
				name: p.name.clone(),
				manufacturer: p.manufacturer.clone(),
				manufacturer_part_number: p.manufacturer_part_number.clone(),
				power: p.power,
			});
		}

		for (i, m) in fx.modules.iter().enumerate() {
			let id =
				ModuleId(u32::try_from(i).map_err(|_| LoadError::TooMany("modules on one hull"))?);
			modules.push((
				id,
				ModuleMeta {
					label: m.label.clone(),
					part: PartId(lookup(&part_ids, &m.part, LoadError::UnknownPart)?),
					serial: m.serial.clone(),
					made: m.made,
				},
				m.condition,
			));
		}

		let mut port_names: Vec<String> = Vec::new();
		let mut seen: BTreeMap<String, PortName> = BTreeMap::new();

		// unlike parts and modules, port names are never declared — they appear
		// inline, repeated, once per connection end. so the table is built by
		// discovery, and `seen` answers the one question that makes it worth
		// doing: have I met this string before?
		let mut intern = |text: &str| -> Result<PortName, LoadError> {
			if let Some(p) = seen.get(text) {
				return Ok(*p);
			}
			let index = u32::try_from(port_names.len())
				.map_err(|_| LoadError::TooMany("distinct port names"))?;
			let p = PortName(index);
			port_names.push(text.to_string());
			seen.insert(text.to_string(), p);
			Ok(p)
		};

		// pass 2: resolution — YOUR connections loop, verbatim
		let mut connections = Vec::new();

		for c in &fx.connections {
			let resolve =
				|label: &str| lookup(&module_ids, label, LoadError::UnknownLabel).map(ModuleId);

			let from = (resolve(&c.from.0)?, intern(&c.from.1)?);
			let to = (resolve(&c.to.0)?, intern(&c.to.1)?);

			connections.push(Connection {
				net: c.net,
				from,
				to,
				run: RunId(lookup(&run_ids, &c.run, LoadError::UnknownRun)?),
			});
		}

		Ok(Self {
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

		let mut world = Self {
			modules,
			condition,
			power,
			parts: ship.parts,
			connections: ship.connections, // …then the original moves. no waste, no assignment
			..Self::new(ship.seed)         // tick, rng, log from the one blessed constructor
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

/// Index a declared list by its name, so references to it can be resolved.
///
/// Position in the list *is* identity — that is what makes the loop counter a
/// valid id. Overflow is a `LoadError` rather than a panic because this runs at
/// an airlock: a fixture with four billion of anything is a malformed fixture,
/// not an impossible state.
fn index_by<T>(
	items: &[T],
	key: impl Fn(&T) -> &str,
	what: &'static str,
) -> Result<BTreeMap<String, u32>, LoadError> {
	items
		.iter()
		.enumerate()
		.map(|(i, t)| {
			let index = u32::try_from(i).map_err(|_| LoadError::TooMany(what))?;
			Ok((key(t).to_string(), index))
		})
		.collect()
}
fn lookup(
	table: &BTreeMap<String, u32>,
	key: &str,
	err: impl Fn(String) -> LoadError,
) -> Result<u32, LoadError> {
	table.get(key).copied().ok_or_else(|| err(key.to_string()))
}
