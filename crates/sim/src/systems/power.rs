use crate::types::catalogue::Part;
use crate::types::events::{EventId, EventKind};
use crate::types::ids::NetworkKind;
use crate::types::modules::ModuleId;
use crate::types::role::PowerRole;
use crate::types::world::World;
use crate::{Condition, ModuleMeta};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

type Adjacency = BTreeMap<ModuleId, Vec<ModuleId>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerState {
	pub(crate) energised: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BreakerState {
	pub(crate) closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceState {
	pub(crate) online: bool,
	charge_anchor: u64,
	anchor_tick: u64,
	depletion_tick: Option<u64>,
	rate_a: u32,
	anchor_cause: Option<EventId>,
}
impl SourceState {
	/// charge(t) = anchor − rate × (t − `anchor_tick`)
	pub(crate) fn charge_at(self, tick: u64) -> u64 {
		let burned = u64::from(self.rate_a).saturating_mul(tick.saturating_sub(self.anchor_tick));
		self.charge_anchor.saturating_sub(burned)
	}
}

#[derive(Debug, Default, PartialEq)]
pub struct PowerNet {
	pub(crate) states: BTreeMap<ModuleId, PowerState>,
	pub(crate) breakers: BTreeMap<ModuleId, BreakerState>,
	pub(crate) sources: BTreeMap<ModuleId, SourceState>,
	pub(crate) pending_trips: BTreeMap<ModuleId, (u64, EventId)>,
}

pub(crate) fn process(world: &mut World) {
	tick_power(world);
	tick_depletion(world);
}

/// Trips scheduled last tick fire now. A thermal breaker is not instantaneous.
pub(crate) fn tick_power(world: &mut World) {
	let due: Vec<ModuleId> = world
		.power
		.pending_trips
		.iter()
		.filter(|(_, (at, _))| *at <= world.tick)
		.map(|(id, _)| *id)
		.collect();

	for id in due {
		let Some((_, cause)) = world.power.pending_trips.remove(&id) else {
			continue;
		};
		let Some(b) = world.power.breakers.get_mut(&id) else {
			continue;
		};
		b.closed = false;
		let ev = world.emit(EventKind::BreakerSet { id, closed: false }, Some(cause));
		settle_power(world, Some(ev));
	}
}
/// Sources whose depletion tick has arrived die before this tick's commands.
/// The burn happened during the interval that just ended; a switch flipped now
/// is too late to save her.
///
pub(crate) fn tick_depletion(world: &mut World) {
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

/// Re-derive who has power, emit the differences, check for overloads.
/// Called after every actuator change, with that change as the cause.
pub(crate) fn settle_power(world: &mut World, cause: Option<EventId>) {
	let adjacency = power_adjacency(world);
	let reached = energised_set(world, &adjacency);
	let announced = emit_power_changes(world, &reached, cause);
	schedule_supply_faults(world, &adjacency, &reached, &announced, cause);
	schedule_overloads(world, &adjacency, &reached, &announced, cause);
	reanchor_sources(world, &adjacency, cause);
}

/// Directed adjacency over Power connections: the fixture wires supply→load, so
/// `from` is always the feeding side. One convention, and propagation and the
/// overload sum become the same walk. Rebuilt per settle — 14 modules; cache
/// when the profiler complains, not before.
fn power_adjacency(world: &World) -> Adjacency {
	let mut adj: Adjacency = BTreeMap::new();
	for c in world
		.connections
		.iter()
		.filter(|c| c.net == NetworkKind::Power)
	{
		adj.entry(c.from.0).or_default().push(c.to.0);
	}
	adj
}

fn energised_set(world: &World, adj: &Adjacency) -> BTreeSet<ModuleId> {
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
			continue; // open breaker: current stops here
		}
		for &next in adj.get(&at).into_iter().flatten() {
			if reached.insert(next) {
				queue.push_back(next);
			}
		}
	}
	reached
}

fn emit_power_changes(
	world: &mut World,
	reached: &BTreeSet<ModuleId>,
	cause: Option<EventId>,
) -> Vec<(ModuleId, EventId)> {
	let changes: Vec<(ModuleId, bool)> = world
		.power
		.states
		.keys()
		.map(|id| (*id, reached.contains(id)))
		.filter(|(id, now)| world.power.states[id].energised != *now)
		.collect();

	let mut announced = Vec::new();
	for (id, energised) in changes {
		world
			.power
			.states
			.get_mut(&id)
			.expect("diffed from this table")
			.energised = energised;
		announced.push((
			id,
			world.emit(EventKind::PowerChanged { id, energised }, cause),
		));
	}
	announced
}

struct OverloadFault {
	id: ModuleId,
	load_a: u32,
	rating_a: u32,
	degraded_rating_a: u32,
	blame: Option<EventId>,
}
/// Overloaded breakers trip now and open next tick — a real thermal breaker
/// does not act instantly, and the delay kills same-tick feedback.
fn schedule_overloads(
	world: &mut World,
	adj: &Adjacency,
	reached: &BTreeSet<ModuleId>,
	announced: &[(ModuleId, EventId)],
	cause: Option<EventId>,
) {
	// plan: every trip decided while borrowing immutably
	let trips: Vec<OverloadFault> = world
		.power
		.breakers
		.iter()
		.filter(|(id, b)| b.closed && reached.contains(id))
		.filter_map(|(id, _)| -> Option<OverloadFault> {
			let Some(PowerRole::Gate { rating_a }) = world.power_role(*id) else {
				return None;
			};
			let condition = world.condition[id];
			let degraded_rating_a = degraded_rating(rating_a, condition);
			let (load_a, downstream) = downstream_load(world, adj, *id);
			let already = world.power.pending_trips.contains_key(id);
			if load_a <= degraded_rating_a || already {
				return None;
			}
			// the last load to come alive under this breaker is what broke it;
			// nothing changed downstream this settle → fall back to the actuator
			let blame = announced
				.iter()
				.rev()
				.find(|(m, _)| downstream.contains(m))
				.map_or(cause, |(_, ev)| Some(*ev));

			Some(OverloadFault {
				id: *id,
				load_a,
				rating_a,
				degraded_rating_a,
				blame,
			})
		})
		.collect();

	// apply
	for fault in trips {
		let ev = world.emit(
			EventKind::BreakerTripped {
				id: fault.id,
				load_a: fault.load_a,
				rating_a: fault.rating_a,
				degraded_rating_a: fault.degraded_rating_a,
			},
			fault.blame,
		);

		world
			.power
			.pending_trips
			.entry(fault.id)
			.or_insert((world.tick + 1, ev));
	}
}

struct SupplyFault {
	id: ModuleId,
	load_a: u32,
	supply_limit_a: u32,
	degraded_rating_a: u32,
	blame: Option<EventId>,
	victims: BTreeSet<ModuleId>,
}

fn schedule_supply_faults(
	world: &mut World,
	adj: &Adjacency,
	reached: &BTreeSet<ModuleId>,
	announced: &[(ModuleId, EventId)],
	cause: Option<EventId>,
) {
	let supply_faults: Vec<SupplyFault> = world
		.modules()
		.iter()
		.filter(|(id, _)| reached.contains(id))
		.filter_map(|(id, _)| -> Option<SupplyFault> {
			let (load_a, downstream) = downstream_load(world, adj, *id);
			let supply_limit_a = world.power_role(*id)?.supply_limit_a()?;
			// let supply_limit_a = supply_limit(meta)?;
			let condition = world.condition[id];
			let degraded_limit_a = degraded_rating(supply_limit_a, condition);
			if load_a <= degraded_limit_a {
				return None;
			}
			let victims = world
				.power
				.breakers
				.iter()
				.filter(|(id, state)| downstream.contains(id) && state.closed)
				.map(|(id, _)| *id)
				.collect();

			let blame = announced
				.iter()
				.rev()
				.find(|(m, _)| downstream.contains(m))
				.map_or(cause, |(_, ev)| Some(*ev));

			Some(SupplyFault {
				id: *id,
				load_a,
				supply_limit_a,
				degraded_rating_a: degraded_limit_a,
				blame,
				victims,
			})
		})
		.collect();

	// apply
	for fault in supply_faults {
		let ev = world.emit(
			EventKind::CapacityExceeded {
				id: fault.id,
				load_a: fault.load_a,
				rating_a: fault.supply_limit_a,
				degraded_rating_a: fault.degraded_rating_a,
			},
			fault.blame,
		);
		for victim in fault.victims {
			world
				.power
				.pending_trips
				.entry(victim)
				.or_insert((world.tick + 1, ev));
		}
	}
}

/// Charge is a formula, so a rate change is the only thing that ever needs
/// writing down. Runs last because the rate must reflect the states this
/// settle just wrote — T+1, deliberately, not T.
fn reanchor_sources(world: &mut World, adj: &Adjacency, cause: Option<EventId>) {
	let now = world.tick;
	let ids: Vec<ModuleId> = world.power.sources.keys().copied().collect();
	let rates: Vec<(ModuleId, u32)> = ids
		.into_iter()
		.map(|id| (id, downstream_load(world, adj, id).0))
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

/// Everything fed by a breaker, and what the energised part of it draws.
/// Follows the arrows, so it cannot walk back up into the bus — no kind
/// heuristics, and a breaker downstream of a breaker sums correctly.
/// Modules past an open breaker are dark, so the energised test excludes them.
fn downstream_load(world: &World, adj: &Adjacency, from: ModuleId) -> (u32, BTreeSet<ModuleId>) {
	let mut seen: BTreeSet<ModuleId> = BTreeSet::new();
	let mut queue = VecDeque::from([from]);
	let mut total = 0;

	while let Some(at) = queue.pop_front() {
		for &next in adj.get(&at).into_iter().flatten() {
			if !seen.insert(next) {
				continue;
			}

			if world.power.states.get(&next).is_some_and(|s| s.energised) {
				total += effective_draw(
					world.power_role(next).map_or(0, PowerRole::draw_a),
					true,
					world.condition[&next],
				);
			}
			queue.push_back(next);
		}
	}
	(total, seen)
}

fn effective_draw(draw_a: u32, energised: bool, condition: Condition) -> u32 {
	if condition.get() == 0.0 || !energised {
		0
	} else {
		draw_a
	}
}

impl PowerNet {
	/// Every module gets its power rows by kind. Exhaustive — a new kind
	/// must state its relationship to electricity before this compiles.
	pub(crate) fn from_modules(parts: &[Part], modules: &BTreeMap<ModuleId, ModuleMeta>) -> Self {
		let mut net = PowerNet::default();
		for (id, meta) in modules {
			match parts[meta.part.0 as usize].power {
				Some(PowerRole::Gate { .. }) => {
					net.breakers.insert(*id, BreakerState { closed: true });
					net.states.insert(*id, PowerState { energised: false });
				}
				Some(PowerRole::Conduit { .. } | PowerRole::Load { .. }) => {
					net.states.insert(*id, PowerState { energised: false });
				}
				Some(PowerRole::Source { capacity, .. }) => {
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
				//Not part of this network
				None => {}
			}
		}
		net
	}
}

//Rating Buffer is what condition value starts to effect the rating of parts
const RATING_BUFFER: f32 = 0.7;

//Its intended to round down to whole amps, so this is fine
#[allow(
	clippy::cast_precision_loss,
	clippy::cast_possible_truncation,
	clippy::cast_sign_loss
)]
fn degraded_rating(rating_a: u32, condition: Condition) -> u32 {
	if condition.get() >= RATING_BUFFER {
		rating_a
	} else {
		(rating_a as f32 * (condition.get() / (RATING_BUFFER))) as u32
	}
}

pub(crate) fn sagging_suppliers(world: &World) -> BTreeSet<ModuleId> {
	let adj = power_adjacency(world);
	world
		.modules
		.keys()
		.filter_map(|id| {
			let limit = world.power_role(*id)?.supply_limit_a()?;
			// let limit = supply_limit(meta)?;
			let (load, seen) = downstream_load(world, &adj, *id);

			let degraded_limit = degraded_rating(limit, world.condition[id]);
			if load * 10 >= degraded_limit * 8 && load > 0 {
				return Some(seen);
			}
			None
		})
		.flatten()
		.collect()
}
pub(crate) fn next_event_at(net: &PowerNet) -> Option<u64> {
	net.sources
		.values()
		.filter_map(|s| s.depletion_tick)
		.chain(net.pending_trips.values().map(|(at, _)| *at))
		.min()
}
