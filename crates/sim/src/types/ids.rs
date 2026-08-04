use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
pub enum NetworkKind {
	Power,
	Data,
	Fluid,
	Thermal,
}

#[cfg(test)]
mod tests {
	use crate::types::modules::ModuleId;

	use std::collections::BTreeMap;

	#[test]
	fn module_ids_order_deterministically() {
		let mut table: BTreeMap<ModuleId, &str> = BTreeMap::new();
		table.insert(ModuleId(3), "pump");
		table.insert(ModuleId(1), "battery");
		table.insert(ModuleId(54), "breaker");

		let ids: Vec<u32> = table.keys().map(|id| id.0).collect();

		assert_eq!(ids, vec![1, 3, 54]);
	}
}
