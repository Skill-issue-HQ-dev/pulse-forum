// The page's small extras: the browser bar colour per screen, square confetti, a tap of vibration, a shake on a
// wrong field, and the live story count that counts up on the start screen.
// Invariant: every effect is optional decoration; with reduced motion or an old phone the flow works the same.

const COLOURS = { lime: "#CBDC3E", coal: "#2B2D2C" };
const CONFETTI = ["#2B2D2C", "#E07D14", "#F2F5F7"];
const still = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

/** The phone's browser bar takes the screen's colour: lime on the cover screens, coal in the flow. */
export function barColour(screen) {
  const meta = document.getElementById("theme-color");
  if (meta) meta.content = screen === "flow" ? COLOURS.coal : COLOURS.lime;
}

/** A short vibration where the phone supports it (Android); iPhones ignore it. */
export function buzz(pattern) {
  if (!still() && navigator.vibrate) navigator.vibrate(pattern);
}

/** Shakes one element once, to point at the field that needs fixing. */
export function shake(el) {
  if (!el || still()) return;
  el.classList.remove("shake");
  void el.offsetWidth;
  el.classList.add("shake");
  el.addEventListener("animationend", () => el.classList.remove("shake"), { once: true });
}

/** Square confetti in the brand's colours, once, then gone. */
export function confetti(box, count = 42) {
  if (!box || still()) return;
  const bits = [];
  for (let i = 0; i < count; i++) {
    const bit = document.createElement("i");
    bit.className = "cf";
    const size = 8 + Math.random() * 9;
    // setProperty, not a style attribute: the page's CSP allows no inline styles.
    const vars = {
      "--x": `${(Math.random() * 100).toFixed(1)}vw`, "--dx": `${((Math.random() - 0.5) * 30).toFixed(1)}vw`,
      "--r": `${Math.round((Math.random() - 0.5) * 900)}deg`, "--s": `${size.toFixed(1)}px`,
      "--c": CONFETTI[i % CONFETTI.length], "--t": `${(1.6 + Math.random() * 1.2).toFixed(2)}s`,
      "--d": `${(Math.random() * 0.35).toFixed(2)}s`,
    };
    for (const [name, value] of Object.entries(vars)) bit.style.setProperty(name, value);
    bits.push(bit);
  }
  box.replaceChildren(...bits);
  setTimeout(() => box.replaceChildren(), 3600);
}

/** Latvian takes the singular after 1, 21, 31 … but not 11. */
export const lvStories = (n) => (n % 10 === 1 && n % 100 !== 11 ? "stāsts" : "stāsti");

/** Counts the number shown in `el` up to `to` over about half a second. */
export function countUp(el, to) {
  const from = Number(el.dataset.n || 0);
  el.dataset.n = String(to);
  if (still() || from === to || from > to) {
    el.textContent = String(to);
    return;
  }
  const start = performance.now();
  const step = (now) => {
    const k = Math.min(1, (now - start) / 550);
    el.textContent = String(Math.round(from + (to - from) * (1 - (1 - k) ** 3)));
    if (k < 1) requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
}
