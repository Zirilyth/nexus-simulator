use crate::{EventKind, ModuleId, RejectReason, World, systems::power::settle_power};

pub(super) fn apply(world: &mut World, id: ModuleId, closed: bool) {
	if let Some(b) = world.power.breakers.get_mut(&id) {
		b.closed = closed;
		let ev = world.emit(EventKind::BreakerSet { id, closed }, None); // paracausal
		settle_power(world, Some(ev));
	} else {
		let reason = if world.modules.contains_key(&id) {
			RejectReason::NotABreaker(id)
		} else {
			RejectReason::NoSuchModule(id)
		};
		world.emit(EventKind::CommandRejected { reason }, None);
	}
}
