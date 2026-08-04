use serde::{Deserialize, Serialize};

use crate::types::condition::Condition;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDef {
	pub label: String,
	pub kind: ModuleKind,
	pub room: String,
	pub maker: String,
	pub part: String,
	pub serial: String,
	pub made: f64,
	pub draw_a: u32,
	pub condition: Condition,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleMeta {
	pub kind: ModuleKind,
	pub label: String,
	pub maker: String,
	pub part: String,
	pub serial: String,
	pub made: f64,
	pub draw_a: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortName(pub String);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ModuleKind {
	BatteryBank { capacity: u64, max_draw_a: u32 },
	Bus { max_a: u32 },
	Breaker { rating_a: u32 },
	Scrubber,
	Heater,
	Pump,
	Sensor,
	Lights,
	Console,
	Valve { open: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ModuleId(pub u32);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_kind_names_itself() {
		// no `_` arm. ADD A VARIANT AND THIS STOPS COMPILING — that's the point.
		fn describe(k: &ModuleKind) -> &'static str {
			match k {
				ModuleKind::BatteryBank { .. } => "battery bank",
				ModuleKind::Bus { .. } => "bus",
				ModuleKind::Breaker { .. } => "breaker",
				ModuleKind::Scrubber => "scrubber",
				ModuleKind::Heater => "heater",
				ModuleKind::Pump => "pump",
				ModuleKind::Sensor => "sensor",
				ModuleKind::Lights => "lights",
				ModuleKind::Console => "console",
				ModuleKind::Valve { .. } => "valve",
			}
		}
		assert_eq!(describe(&ModuleKind::Breaker { rating_a: 25 }), "breaker");
	}
}
