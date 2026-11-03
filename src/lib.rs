//! WoW: Forever druid levelling 1-60. The client's tables (`client`), items
//! (`items`), talent trees (`talents`), the levelling route machinery (`route`)
//! and the numbers the client does not hold (`assumptions`); the druid's sims
//! in `druid`.

pub mod assumptions;
pub mod client;
pub mod druid;
pub mod items;
pub mod route;
pub mod talents;
