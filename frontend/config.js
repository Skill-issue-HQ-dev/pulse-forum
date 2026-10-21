// The one address this site talks to: the collector behind dbautomatizacijas.com/pulse.
// Owns nothing else; the guest page and the organisers' page both import it.
// Invariant: changing it is the only way to point the site elsewhere (tests/frontend_contract.rs checks it).

export const API = "https://dbautomatizacijas.com/pulse/api";
