#[cfg(feature = "gateway")]
pub mod auth;
#[cfg(feature = "gateway")]
pub mod daemon;
#[cfg(feature = "gateway")]
pub mod routes;
#[cfg(feature = "gateway")]
pub mod ws;

#[cfg(feature = "gateway")]
pub use daemon::start_gateway;
