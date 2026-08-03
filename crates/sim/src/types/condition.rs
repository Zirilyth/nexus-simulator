use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f32")]
pub struct Condition(f32);

impl Condition {
	pub fn new(v: f32) -> Result<Self, InvalidCondition> {
		if (0.0..=1.0).contains(&v) && v.is_finite() {
			Ok(Condition(v))
		} else {
			Err(InvalidCondition(v))
		}
	}
	pub fn get(self) -> f32 {
		self.0
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InvalidCondition(pub f32);

impl std::fmt::Display for InvalidCondition {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "condition {} outside 0.0..=1.0 (or not finite)", self.0)
	}
}

impl std::error::Error for InvalidCondition {}

impl TryFrom<f32> for Condition {
	type Error = InvalidCondition;

	fn try_from(v: f32) -> Result<Self, Self::Error> {
		Condition::new(v)
	}
}
