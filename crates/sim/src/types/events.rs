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
}
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
	Wait,
}
