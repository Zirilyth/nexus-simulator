use crate::ModuleMeta;
use crate::types::events::{EventId, EventKind};
use crate::types::ids::NetworkKind;
use crate::types::modules::{ModuleId, ModuleKind};
use crate::types::world::World;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerState {
	pub energised: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakerState {
	pub closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceState {
	pub online: bool,
}

#[derive(Debug, Default)]
pub struct PowerNet {
	pub states: BTreeMap<ModuleId, PowerState>,
	pub breakers: BTreeMap<ModuleId, BreakerState>,
	pub sources: BTreeMap<ModuleId, SourceState>,
	pub pending_trips: Vec<(ModuleId, EventId)>,
}

/// Trips scheduled last tick fire now. A thermal breaker is not instantaneous.
pub fn tick_power(world: &mut World) {
	let pending = std::mem::take(&mut world.power.pending_trips);
	for (id, trip_event) in pending {
		if let Some(b) = world.power.breakers.get_mut(&id) {
			b.closed = false;
		}
		settle_power(world, Some(trip_event));
	}
}

/// Re-derive who has power, emit the differences, check for overloads.
/// Called after every actuator change, with that change as the cause.
pub(crate) fn settle_power(world: &mut World, cause: Option<EventId>) {
	// ---- plan phase: read everything, borrow nothing across the apply ----
	let adjacency = power_adjacency(world);

	// BFS from every online source; enter breakers only if closed
	let mut reached: BTreeSet<ModuleId> = BTreeSet::new();
	let mut queue: VecDeque<ModuleId> = world
		.power
		.sources
		.iter()
		.filter(|(_, s)| s.online)
		.map(|(id, _)| *id)
		.collect();
	reached.extend(queue.iter().copied());

	while let Some(at) = queue.pop_front() {
		if let Some(b) = world.power.breakers.get(&at)
			&& !b.closed
		{
			continue;
		} // open breaker: current stops here
		for &next in adjacency.get(&at).into_iter().flatten() {
			if reached.insert(next) {
				queue.push_back(next);
			}
		}
	}

	// diff against current states — the plan
	let changes: Vec<(ModuleId, bool)> = world
		.power
		.states
		.keys()
		.map(|id| (*id, reached.contains(id)))
		.filter(|(id, now)| world.power.states[id].energised != *now)
		.collect();

	// ---- apply phase: no outstanding borrows, mutate freely ----
	for (id, energised) in changes {
		world
			.power
			.states
			.get_mut(&id)
			.expect("diffed from this table")
			.energised = energised;
		world.emit(EventKind::PowerChanged { id, energised }, cause);
	}

	// ---- overload check: schedule trips for next tick ----
	let breaker_ids: Vec<ModuleId> = world
		.power
		.breakers
		.iter()
		.filter(|(id, b)| b.closed && reached.contains(id))
		.map(|(id, _)| *id)
		.collect();

	for id in breaker_ids {
		let load_a = downstream_draw(world, &adjacency, id);
		let ModuleKind::Breaker { rating_a } = world.modules[&id].kind else {
			continue;
		};
		let already = world.power.pending_trips.iter().any(|(p, _)| *p == id);
		if load_a > rating_a && !already {
			let ev = world.emit(
				EventKind::BreakerTripped {
					id,
					load_a,
					rating_a,
				},
				cause,
			);
			world.power.pending_trips.push((id, ev));
		}
	}
}

/// Undirected adjacency over Power connections. Rebuilt per settle — 14 modules;
/// cache when the profiler complains, not before.
fn power_adjacency(world: &World) -> BTreeMap<ModuleId, Vec<ModuleId>> {
	let mut adj: BTreeMap<ModuleId, Vec<ModuleId>> = BTreeMap::new();
	for c in world
		.connections
		.iter()
		.filter(|c| c.net == NetworkKind::Power)
	{
		adj.entry(c.from.0).or_default().push(c.to.0);
		adj.entry(c.to.0).or_default().push(c.from.0);
	}
	adj
}

/// Sum of energised load draw downstream of a breaker: its component after
/// refusing to walk back into buses, sources, or other breakers.
/// Honest for radial wiring; mesh wiring is a later.md problem.
fn downstream_draw(
	world: &World,
	adj: &BTreeMap<ModuleId, Vec<ModuleId>>,
	breaker: ModuleId,
) -> u32 {
	let mut seen = BTreeSet::from([breaker]);
	let mut queue = VecDeque::from([breaker]);
	let mut total = 0;

	while let Some(at) = queue.pop_front() {
		for &next in adj.get(&at).into_iter().flatten() {
			let kind = &world.modules[&next].kind;
			let upstream = matches!(
				kind,
				ModuleKind::Bus | ModuleKind::BatteryBank | ModuleKind::Breaker { .. }
			);
			if upstream || !seen.insert(next) {
				continue;
			}
			if world.power.states[&next].energised {
				total += world.modules[&next].power_draw;
			}
			queue.push_back(next);
		}
	}
	total
}

impl PowerNet {
	/// Every module gets its power rows by kind. Exhaustive — a new kind
	/// must state its relationship to electricity before this compiles.
	pub fn from_modules(modules: &BTreeMap<ModuleId, ModuleMeta>) -> Self {
		let mut net = PowerNet::default();
		for (id, meta) in modules {
			match meta.kind {
				ModuleKind::BatteryBank => {
					net.sources.insert(*id, SourceState { online: false });
					net.states.insert(*id, PowerState { energised: false });
				}
				ModuleKind::Breaker { .. } => {
					net.breakers.insert(*id, BreakerState { closed: true });
					net.states.insert(*id, PowerState { energised: false });
				}
				ModuleKind::Bus
				| ModuleKind::Scrubber
				| ModuleKind::Heater
				| ModuleKind::Pump
				| ModuleKind::Sensor
				| ModuleKind::Lights
				| ModuleKind::Console => {
					net.states.insert(*id, PowerState { energised: false });
				}
				ModuleKind::Valve { .. } => {} // fluid citizen; joins its own net in phase 3
			}
		}
		net
	}
}
