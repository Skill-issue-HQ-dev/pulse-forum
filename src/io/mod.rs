//! Adapters at the edge: the entries database, the clock, the operating system's randomness and the site's files.
//! Owns every read and write outside the process; core never calls into this layer.

pub mod clock;
pub mod files;
pub mod random;
pub mod store;
