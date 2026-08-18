use crate::{Condition, EventKind, ModuleId, RejectReason, World, systems::power::settle_power};

pub(super) fn apply(world: &mut World, id: ModuleId, new_condition: Condition) {
	if let Some(current) = world.condition.get_mut(&id) {
		let old_condition = current.get();
		*current = new_condition;
		let ev = world.emit(
			EventKind::ConditionChanged {
				id,
				from: old_condition,
				to: new_condition.get(),
			},
			None,
		);
		settle_power(world, Some(ev));
	} else {
		world.emit(
			EventKind::CommandRejected {
				reason: RejectReason::NoSuchModule(id),
			},
			None,
		);
	}
}
