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
	charge_anchor: u64,
	anchor_tick: u64,
	depletion_tick: Option<u64>,
	rate_a: u32,
	anchor_cause: Option<EventId>,
}
impl SourceState {
	pub(crate) fn charge_at(self, tick: u64) -> u64 {
		let burned = u64::from(self.rate_a) * tick.saturating_sub(self.anchor_tick);
		self.charge_anchor.saturating_sub(burned)
	}
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
	// keep what we emitted: a trip's cause is the PowerChanged that overloaded it
	let mut announced: Vec<(ModuleId, EventId)> = Vec::new();
	for (id, energised) in changes {
		world
			.power
			.states
			.get_mut(&id)
			.expect("diffed from this table")
			.energised = energised;
		let ev = world.emit(EventKind::PowerChanged { id, energised }, cause);
		announced.push((id, ev));
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
		let (load_a, downstream) = downstream_load(world, &adjacency, id);
		let ModuleKind::Breaker { rating_a } = world.modules[&id].kind else {
			continue;
		};
		let already = world.power.pending_trips.iter().any(|(p, _)| *p == id);
		if load_a > rating_a && !already {
			// the last load to come alive under this breaker is what broke it.
			// nothing changed downstream this settle → fall back to the actuator.
			let blame = announced
				.iter()
				.rev()
				.find(|(m, _)| downstream.contains(m))
				.map_or(cause, |(_, ev)| Some(*ev));
			let ev = world.emit(
				EventKind::BreakerTripped {
					id,
					load_a,
					rating_a,
				},
				blame,
			);
			world.power.pending_trips.push((id, ev));
		}
	}
	let now = world.tick;
	let rates: Vec<(ModuleId, u32)> = world
		.power
		.sources
		.keys()
		.copied()
		.collect::<Vec<_>>()
		.into_iter()
		.map(|id| (id, downstream_load(world, &adjacency, id).0))
		.collect();

	for (id, rate_a) in rates {
		let s = world
			.power
			.sources
			.get_mut(&id)
			.expect("planned from this table");

		if s.rate_a != rate_a {
			s.charge_anchor = s.charge_at(now);
			s.anchor_tick = now;
			s.rate_a = rate_a;
			s.anchor_cause = cause;
		}

		s.depletion_tick = if s.online && s.rate_a > 0 {
			Some(s.anchor_tick + s.charge_anchor / u64::from(s.rate_a))
		} else {
			None
		};
	}
}

/// Directed adjacency over Power connections: the fixture wires supply→load, so
/// `from` is always the feeding side. One convention, and propagation and the
/// overload sum become the same walk. Rebuilt per settle — 14 modules; cache
/// when the profiler complains, not before.
fn power_adjacency(world: &World) -> BTreeMap<ModuleId, Vec<ModuleId>> {
	let mut adj: BTreeMap<ModuleId, Vec<ModuleId>> = BTreeMap::new();
	for c in world
		.connections
		.iter()
		.filter(|c| c.net == NetworkKind::Power)
	{
		adj.entry(c.from.0).or_default().push(c.to.0);
	}
	adj
}

/// Sources whose depletion tick has arrived die before this tick's commands.
/// The burn happened during the interval that just ended; a switch flipped now
/// is too late to save her.
pub fn tick_depletion(world: &mut World) {
	let now = world.tick;
	let dead: Vec<(ModuleId, Option<EventId>)> = world
		.power
		.sources
		.iter()
		.filter(|(_, s)| s.online && s.depletion_tick.is_some_and(|d| now >= d))
		.map(|(id, s)| (*id, s.anchor_cause))
		.collect();

	for (id, anchor_cause) in dead {
		let s = world
			.power
			.sources
			.get_mut(&id)
			.expect("planned from this table");
		s.online = false;
		s.charge_anchor = 0;
		s.rate_a = 0;
		s.depletion_tick = None;
		// cause: the flip that fixed her final burn rate. walk it back and it
		// still ends at a paracausal None.
		let ev = world.emit(EventKind::SourceDepleted { id }, anchor_cause);
		settle_power(world, Some(ev));
	}
}

/// Everything fed by a breaker, and what the energised part of it draws.
/// Follows the arrows, so it cannot walk back up into the bus — no kind
/// heuristics, and a breaker downstream of a breaker sums correctly.
/// Modules past an open breaker are dark, so the energised test excludes them.
fn downstream_load(
	world: &World,
	adj: &BTreeMap<ModuleId, Vec<ModuleId>>,
	breaker: ModuleId,
) -> (u32, BTreeSet<ModuleId>) {
	let mut seen: BTreeSet<ModuleId> = BTreeSet::new();
	let mut queue = VecDeque::from([breaker]);
	let mut total = 0;

	while let Some(at) = queue.pop_front() {
		for &next in adj.get(&at).into_iter().flatten() {
			if !seen.insert(next) {
				continue;
			}
			if world.power.states.get(&next).is_some_and(|s| s.energised) {
				total += world.modules[&next].draw_a;
			}
			queue.push_back(next);
		}
	}
	(total, seen)
}

impl PowerNet {
	/// Every module gets its power rows by kind. Exhaustive — a new kind
	/// must state its relationship to electricity before this compiles.
	pub fn from_modules(modules: &BTreeMap<ModuleId, ModuleMeta>) -> Self {
		let mut net = PowerNet::default();
		for (id, meta) in modules {
			match meta.kind {
				ModuleKind::BatteryBank { capacity } => {
					net.sources.insert(
						*id,
						SourceState {
							online: false,
							charge_anchor: capacity,
							anchor_tick: 0,
							depletion_tick: None,
							rate_a: 0,
							anchor_cause: None,
						},
					);
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
