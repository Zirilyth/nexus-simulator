use crate::{Command, World};

mod set_breaker;
mod set_condition;
mod set_source;

/// Apply each command, then let the consequences settle behind it.
pub(crate) fn apply_commands(world: &mut World, commands: &[Command]) {
	for cmd in commands {
		match *cmd {
			Command::SetBreaker { id, closed } => set_breaker::apply(world, id, closed),
			Command::SetSource { id, online } => set_source::apply(world, id, online),
			Command::SetCondition { id, new_condition } => {
				set_condition::apply(world, id, new_condition);
			}
		}
	}
}
