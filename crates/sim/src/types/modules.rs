use serde::{Deserialize, Serialize};

use crate::types::{catalogue::PartId, condition::Condition};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDef {
	pub label: String,
	pub room: String,
	pub part: String,
	pub serial: String,
	pub made: f64,
	pub condition: Condition,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleMeta {
	pub label: String,
	pub part: PartId,
	pub serial: String,
	pub made: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PortName(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Names a module, which may or may not be aboard.
///
/// Commands carry these in from scripts, savegames and mods, so "no such
/// module" is a supported answer rather than an impossible one — and anything
/// that resolves a `ModuleId` against a world has to say so in its return type.
pub struct ModuleId(pub u32);

impl ModuleId {
	#[must_use]
	pub const fn index(self) -> u32 {
		self.0
	}
}
