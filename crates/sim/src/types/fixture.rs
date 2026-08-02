use serde::{Deserialize, Serialize};

use crate::types::ids::NetworkKind;
use crate::types::meta::ModuleKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipFixture {
	pub class: String,
	pub hull_no: String,
	pub commissioned: f64,
	pub seed: u64,
	pub rooms: Vec<RoomDef>,
	pub runs: Vec<RunDef>,
	pub modules: Vec<ModuleDef>,
	pub connections: Vec<ConnDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDef {
	pub id: String,
	pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunDef {
	pub id: String,
	pub cap: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDef {
	pub label: String,
	pub kind: ModuleKind,
	pub room: String,
	pub maker: String,
	pub part: String,
	pub serial: String,
	pub made: f64,
	pub condition: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnDef {
	pub net: NetworkKind,
	pub from: (String, String),
	pub to: (String, String),
	pub run: String,
}
