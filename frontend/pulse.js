// The PULSE logo's line reaching the bottom of its screen, and the pulse along it: a light signal flowing down,
// right along the L's foot and down again, twice in a row like a heartbeat [ba-dum], then a rest.
// Invariant: the line's lower end follows the screen's bottom; the pulse is decoration only, never under reduced motion.

const NS = "http://www.w3.org/2000/svg";
const ORANGE = "#E07D14";
/** The tail's fade behind the head, step by step. */
const TAIL = Array.from({ length: 12 }, (_, k) => (1 - k / 12) ** 2);
/** Layer j covers the first j+1 steps; these opacities stack up to TAIL without seams between the steps. */
const LAYERS = TAIL.map((a, k) => (k === TAIL.length - 1 ? a : 1 - (1 - a) / (1 - TAIL[k + 1])));
/** The L-line in the mark's own units: line 1's centre, the step's height, line 2's centre and top. */
const L1X = 343.46;
const STEP_Y = 563.39;
const L2X = 380.83;
const L2TOP = 535.92;

/** How each mark beats. `top` is where the pulse starts [above the mark]; it moves at `speed` units a second,
 *  then rests; `beats` rounds, then the line rests for good, so a phone left on a screen is not kept busy. */
const KINDS = {
  hero: { top: -150, core: 20, seg: 12, halo: [[44, 0.16], [68, 0.09], [96, 0.05]], speed: 700, rest: 2.9, begin: 1.25, echo: 0.32, beats: 10 },
  rail: { top: 100, core: 30, seg: 48, halo: [[70, 0.2], [110, 0.11], [150, 0.05]], speed: 3230, rest: 2.8, begin: 1.6, echo: 0.45, beats: 12 },
};

const el = (name, attrs) => {
  const node = document.createElementNS(NS, name);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, String(v));
  return node;
};

/** The zig-zag from `top` down to `end`, and its length. */
const zigzag = (top, end) => ({ d: `M${L1X} ${top}V${STEP_Y}H${L2X}V${end}`, len: STEP_Y - top + (L2X - L1X) + (end - STEP_Y) });

/** Where line 2 must end, in the mark's units, to reach the bottom of the screen the mark sits on. */
function bottomOf(svg) {
  const screen = svg.closest("section, form");
  const vb = svg.viewBox.baseVal;
  const box = svg.getBoundingClientRect();
  if (!screen || !box.width) return vb.y + vb.height;
  const units = (screen.getBoundingClientRect().bottom - box.top) * (vb.width / box.width);
  return Math.max(vb.y + vb.height, Math.ceil(vb.y + units));
}

/** Stretches the drawn line and its clip to `end`; the drawing keeps its place [pathLength is 1]. */
function stretch(svg, line, end) {
  const lower = svg.querySelector("clipPath rect:last-of-type");
  if (lower) lower.setAttribute("height", String(end - L2TOP + 2));
  line.setAttribute("d", `M${L1X} ${svg.viewBox.baseVal.y}V${STEP_Y}H${L2X}V${end}`);
}

/** One dash whose front runs from the path's start to past its end, eased, then rests until the next beat. */
function moving(path, timing, { length, width, opacity, ahead = 0, cap = "butt" }) {
  const from = length - ahead;
  const to = from - (path.len + (TAIL.length + 2) * timing.seg);
  const node = el("path", {
    d: path.d, fill: "none", stroke: ORANGE, "stroke-width": width, opacity, "stroke-linecap": cap,
    "stroke-dasharray": `${length} 60000`, "stroke-dashoffset": from,
  });
  node.append(el("animate", {
    attributeName: "stroke-dashoffset", values: `${from};${to};${to}`,
    keyTimes: `0;${(timing.travel / timing.period).toFixed(4)};1`, calcMode: "spline",
    keySplines: "0.45 0 0.25 1;0 0 1 1", dur: `${timing.period.toFixed(2)}s`, begin: `${timing.at.toFixed(2)}s`,
    repeatCount: timing.beats,
  }));
  return node;
}

/** A comet: a soft glow under the mark, and inside the line a round head with a fading tail. The fill-less rect
 *  over the whole mark keeps WebKit from clipping the comet to its path's narrow box. */
function comet(kind, path, timing, strength, clipId, box) {
  const halo = el("g", { class: "mk-comet", opacity: strength });
  for (const [width, opacity] of kind.halo) {
    halo.append(moving(path, timing, { length: kind.seg * 2, width, opacity, ahead: kind.seg * 0.4, cap: "round" }));
  }
  const core = el("g", { class: "mk-comet", opacity: strength, "clip-path": `url(#${clipId})` });
  core.append(el("rect", { x: box.x, y: box.y - 400, width: box.width, height: path.len + 800, fill: "none" }));
  LAYERS.forEach((opacity, j) => core.append(moving(path, timing, { length: kind.seg * (j + 1), width: kind.core, opacity })));
  core.append(moving(path, timing, { length: 0.01, width: kind.core, opacity: 1, cap: "round" }));
  return { halo, core };
}

/** Fits every mark of `root` to its screen and starts the pulse, timed from now; call it whenever that screen shows
 *  or changes height. A fit that moved the line's end less than a few units leaves a running pulse alone. */
export function startPulse(root, { force = true } = {}) {
  if (!root) return;
  const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  for (const svg of root.querySelectorAll("svg[data-pulse]")) {
    const kind = KINDS[svg.dataset.pulse];
    const clip = svg.querySelector("clipPath");
    const letters = svg.querySelector(".mk-letters");
    const line = svg.querySelector(".mk-zz");
    if (!kind || !clip || !letters || !line) continue;
    const end = bottomOf(svg);
    if (!force && Math.abs(end - Number(svg.dataset.end || 0)) < 4) continue;
    svg.dataset.end = String(end);
    stretch(svg, line, end);
    for (const old of svg.querySelectorAll(".mk-comet")) old.remove();
    if (still) continue;
    const path = zigzag(kind.top, end + 120);
    const travel = path.len / kind.speed;
    const timing = { seg: kind.seg, travel, period: travel + kind.rest, beats: kind.beats, at: svg.getCurrentTime() + kind.begin };
    const box = svg.viewBox.baseVal;
    const first = comet(kind, path, timing, 1, clip.id, box);
    const echo = comet(kind, path, { ...timing, at: timing.at + kind.echo }, 0.65, clip.id, box);
    letters.before(first.halo, echo.halo);
    line.parentNode.after(first.core, echo.core);
  }
}
