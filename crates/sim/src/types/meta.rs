use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleMeta {
	pub kind: ModuleKind,
	pub label: String,
	pub maker: String,
	pub part: String,
	pub serial: String,
	pub made: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum ModuleKind {
	BatteryBank,
	Bus,
	Breaker { rating_a: u32 },
	Scrubber,
	Heater,
	Pump,
	Sensor,
	Lights,
	Console,
	Valve { open: bool },
}
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn every_kind_names_itself() {
		// no `_` arm. ADD A VARIANT AND THIS STOPS COMPILING — that's the point.
		fn describe(k: &ModuleKind) -> &'static str {
			match k {
				ModuleKind::BatteryBank => "battery bank",
				ModuleKind::Bus => "bus",
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
