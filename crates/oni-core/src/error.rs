pub use color_eyre::eyre;
pub use eyre::eyre as err;
pub use eyre::WrapErr;

pub type Result<T> = eyre::Result<T>;
