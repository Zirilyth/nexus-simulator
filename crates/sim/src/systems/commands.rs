use crate::{Command, EventKind, World, types::events::RejectReason};

pub fn apply_commands(world: &mut World, commands: &[Command]) {
	for cmd in commands {
		match *cmd {
			Command::SetBreaker { id, closed } => {
				match world.power.breakers.get_mut(&id) {
					Some(b) => {
						b.closed = closed;
						world.emit(EventKind::BreakerSet { id, closed }, None); // paracausal
					}
					None => {
						let reason = if world.modules.contains_key(&id) {
							RejectReason::NotABreaker(id)
						} else {
							RejectReason::NoSuchModule(id)
						};
						world.emit(EventKind::CommandRejected { reason }, None);
					}
				}
			}
			Command::SetSource { id, online } => match world.power.sources.get_mut(&id) {
				Some(s) => {
					s.online = online;
					world.emit(EventKind::SourceSet { id, online }, None);
				}
				None => {
					let reason = if world.modules.contains_key(&id) {
						RejectReason::NotASource(id)
					} else {
						RejectReason::NoSuchModule(id)
					};
					world.emit(EventKind::CommandRejected { reason }, None);
				}
			},
		}
	}
}
