use crate::{EventKind, ModuleId, RejectReason, World, systems::power::settle_power};

pub(super) fn apply(world: &mut World, id: ModuleId, online: bool) {
	if let Some(s) = world.power.sources.get_mut(&id) {
		s.online = online;
		let ev = world.emit(EventKind::SourceSet { id, online }, None); // paracausal
		settle_power(world, Some(ev));
	} else {
		let reason = if world.modules.contains_key(&id) {
			RejectReason::NotASource(id)
		} else {
			RejectReason::NoSuchModule(id)
		};
		world.emit(EventKind::CommandRejected { reason }, None);
	}
}
