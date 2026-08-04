use crate::types::modules::ModuleId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventId(pub u64);

#[derive(Debug, Clone, PartialEq)]
pub struct Event {
	pub id: EventId,
	pub at_tick: u64,
	pub cause: Option<EventId>,
	pub kind: EventKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventKind {
	WorldLoaded,
	BreakerSet {
		id: ModuleId,
		closed: bool,
	},
	SourceSet {
		id: ModuleId,
		online: bool,
	},
	PowerChanged {
		id: ModuleId,
		energised: bool,
	},
	BreakerTripped {
		id: ModuleId,
		load_a: u32,
		rating_a: u32,
	},
	CapacityExceeded {
		id: ModuleId,
		load_a: u32,
		rating_a: u32,
	},
	CommandRejected {
		reason: RejectReason,
	},
	SourceDepleted {
		id: ModuleId,
	},
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
	SetBreaker { id: ModuleId, closed: bool },
	SetSource { id: ModuleId, online: bool },
}
#[derive(Debug, Clone, PartialEq)]
pub enum RejectReason {
	NoSuchModule(ModuleId),
	NotABreaker(ModuleId),
	NotASource(ModuleId),
}
