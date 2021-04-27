use thiserror::Error;

#[derive(Clone, Copy, Debug, Error)]
pub enum Executor {
	#[error("Executor was shutdown before finishing")]
	Cancelled,
}
