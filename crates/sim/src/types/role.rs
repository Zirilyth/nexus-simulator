use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerRole {
	Source { capacity: u64, max_draw_a: u32 },
	Conduit { max_a: u32 },
	Gate { rating_a: u32 },
	Load { draw_a: u32 },
}
impl PowerRole {
	#[must_use]
	pub const fn draw_a(self) -> u32 {
		match self {
			Self::Source { .. } | Self::Conduit { .. } | Self::Gate { .. } => 0,
			Self::Load { draw_a } => draw_a,
		}
	}
	#[must_use]
	pub const fn supply_limit_a(self) -> Option<u32> {
		match self {
			Self::Source { max_draw_a, .. } => Some(max_draw_a),
			Self::Conduit { max_a } => Some(max_a),
			Self::Gate { .. } | Self::Load { .. } => None,
		}
	}
}

//pub enum FluidRole {}

//pub enum AtmosphericsRole {}
