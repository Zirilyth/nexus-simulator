use serde::{Deserialize, Serialize};

use crate::types::role::PowerRole;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Part {
	pub name: String,
	pub manufacturer_part_number: String,
	pub manufacturer: String,
	pub power: Option<PowerRole>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct PartId(pub u32);
