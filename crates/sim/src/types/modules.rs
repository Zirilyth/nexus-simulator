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
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortName(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ModuleId(pub u32);
