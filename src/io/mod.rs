//! Adapters at the edge: the entries database, the clock and the operating system's randomness.
//! Owns every read and write outside the process; core never calls into this layer.

pub mod clock;
pub mod random;
pub mod store;
