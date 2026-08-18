use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PowerRole {
	Source { capacity: u64, max_draw_a: u32 },
	Conduit { max_a: u32 },
	Gate { rating_a: u32 },
	Load { draw_a: u32 },
}
impl PowerRole {
	#[must_use]
	pub fn draw_a(self) -> u32 {
		match self {
			PowerRole::Source { .. } | PowerRole::Conduit { .. } | PowerRole::Gate { .. } => 0,
			PowerRole::Load { draw_a } => draw_a,
		}
	}
	#[must_use]
	pub fn supply_limit_a(self) -> Option<u32> {
		match self {
			PowerRole::Source { max_draw_a, .. } => Some(max_draw_a),
			PowerRole::Conduit { max_a } => Some(max_a),
			PowerRole::Gate { .. } | PowerRole::Load { .. } => None,
		}
	}
}

//pub enum FluidRole {}

//pub enum AtmosphericsRole {}
