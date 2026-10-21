//! The wall clock as the store and the API use it.
//! Owns reading the clock only; Riga time for people is derived in core::riga.
//! Invariant: instants are RFC 3339 UTC strings in whole seconds, like "2026-10-28T10:15:02Z".

/// Now, as RFC 3339 UTC without fractional seconds.
pub fn now() -> String {
    let ts = jiff::Timestamp::now();
    jiff::Timestamp::from_second(ts.as_second()).unwrap_or(ts).to_string()
}
