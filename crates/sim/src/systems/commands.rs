use crate::systems::power::settle_power;
use crate::{Command, EventKind, World, types::events::RejectReason};

/// Apply each command, then let the consequences settle behind it.
pub fn apply_commands(world: &mut World, commands: &[Command]) {
	for cmd in commands {
		match *cmd {
			Command::SetBreaker { id, closed } => {
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
			Command::SetSource { id, online } => {
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
			Command::SetCondition { id, new_condition } => {
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
		}
	}
}
