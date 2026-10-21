// The entry rules the page checks before sending, the same ones the collector applies again.
// Owns field limits and shape checks; returns error keys from strings.js, never text.
// Invariant: lengths count characters, not bytes, so ā or an emoji is one, as on the server.

export const LIMITS = { storyMin: 20, storyMax: 1500, name: 80, email: 120, company: 120, phoneDigitsMin: 7, phoneDigitsMax: 15 };

export const chars = (s) => Array.from(s).length;

/** One @, something before it, a dot after it with two characters behind, no spaces. */
export function emailOk(email) {
  if (/\s/.test(email)) return false;
  const parts = email.split("@");
  if (parts.length !== 2 || parts[0] === "") return false;
  const domain = parts[1];
  for (let i = 1; i < domain.length; i++) {
    if (domain[i] === "." && domain.length - i - 1 >= 2) return true;
  }
  return false;
}

/** Optional leading +, then digits, spaces and - ( ) . only; 7 to 15 digits. */
export function phoneOk(phone) {
  if (!/^\+?[0-9 ()\-.]+$/.test(phone)) return false;
  const digits = phone.replace(/\D/g, "").length;
  return digits >= LIMITS.phoneDigitsMin && digits <= LIMITS.phoneDigitsMax;
}

export function storyError(story) {
  const n = chars(story.trim());
  if (n < LIMITS.storyMin) return "errStoryShort";
  if (n > LIMITS.storyMax) return "errStoryLong";
  return null;
}

/** Field errors of the contact step as { field: errorKey }; empty when the step is fine. */
export function contactErrors(v) {
  const e = {};
  for (const f of ["first", "last", "email", "phone"]) {
    if (v[f].trim() === "") e[f] = "errMissing";
  }
  if (!e.first && chars(v.first.trim()) > LIMITS.name) e.first = "errTooLong";
  if (!e.last && chars(v.last.trim()) > LIMITS.name) e.last = "errTooLong";
  if (!e.email && (chars(v.email.trim()) > LIMITS.email || !emailOk(v.email.trim()))) e.email = "errEmail";
  if (!e.phone && !phoneOk(v.phone.trim())) e.phone = "errPhone";
  if (chars(v.company.trim()) > LIMITS.company) e.company = "errTooLong";
  if (!v.consent) e.consent = "errConsent";
  return e;
}

/** The collector's 422 codes, mapped to the step and field that show them. */
export const SERVER_CODES = {
  category: { step: 1, field: "category", key: "errPick" },
  story_short: { step: 2, field: "story", key: "errStoryShort" },
  story_long: { step: 2, field: "story", key: "errStoryLong" },
  missing: { step: 3, field: "first", key: "errMissing" },
  too_long: { step: 3, field: "first", key: "errTooLong" },
  email: { step: 3, field: "email", key: "errEmail" },
  phone: { step: 3, field: "phone", key: "errPhone" },
  consent: { step: 3, field: "consent", key: "errConsent" },
};

/** 16 random bytes as 32 lowercase hex: one code per entry, so a resend is never a second entry. */
export function newToken() {
  const b = new Uint8Array(16);
  crypto.getRandomValues(b);
  return Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");
}
