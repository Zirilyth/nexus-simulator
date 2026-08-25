use std::collections::BTreeMap;

use crate::{World, advance_to, types::ids::ShipId};

#[derive(Debug, PartialEq)]
pub struct Universe {
	pub(crate) worlds: BTreeMap<ShipId, World>,
}

impl Universe {
	#[must_use]
	pub fn next_event_at(&self) -> Option<u64> {
		self.worlds
			.values()
			.filter_map(super::world::World::next_event_at)
			.min()
	}

	pub fn advance_to(&mut self, tick: u64) {
		for w in self.worlds.values_mut() {
			advance_to(w, tick);
			// events stay in each world's log — advancing the universe is a state
			// question, not a stream one. a caller that wants the merged stream needs
			// to decide chronological-vs-grouped ordering first; nothing does yet.
			let () = ();
		}
	}

	#[must_use]
	pub fn new() -> Universe {
		let empty_list: BTreeMap<ShipId, World> = BTreeMap::new();
		Universe { worlds: empty_list }
	}

	pub fn add_world(&mut self, world: World) -> ShipId {
		let id = ShipId(
			self.worlds
				.last_key_value()
				.map_or(0, |(last, _)| last.0 + 1),
		);
		self.worlds.insert(id, world);
		id
	}

	#[must_use]
	pub fn world(&self, id: ShipId) -> Option<&World> {
		self.worlds.get(&id)
	}
	#[must_use]
	pub fn world_mut(&mut self, id: ShipId) -> Option<&mut World> {
		self.worlds.get_mut(&id)
	}
}

impl Default for Universe {
	fn default() -> Self {
		Self::new()
	}
}
