// The PULSE logo's line spanning its whole screen, and the pulse along it: a light signal flowing down, right along
// the L's foot and down again, twice in a row like a heartbeat [ba-dum], then a rest.
// Invariant: the line runs from the screen's top edge to its bottom edge whatever the screen's size [refit whenever a
// screen changes size]; the pulse is decoration only, never under reduced motion.

const NS = "http://www.w3.org/2000/svg";
const ORANGE = "#E07D14";
/** The tail's fade behind the head, step by step. */
const TAIL = Array.from({ length: 20 }, (_, k) => (1 - k / 20) ** 2);
/** Layer j covers the first j+1 steps; these opacities stack up to TAIL without seams between the steps. */
const LAYERS = TAIL.map((a, k) => (k === TAIL.length - 1 ? a : 1 - (1 - a) / (1 - TAIL[k + 1])));
/** The L-line in the mark's own units: line 1's centre, the step's height, line 2's centre and top. */
const L1X = 343.46;
const STEP_Y = 563.39;
const L2X = 380.83;
const L2TOP = 535.92;

/** How each mark beats. The pulse starts `lead` units above the line's top; it moves at `speed` units a second,
 *  then rests; `beats` rounds, then the line rests for good, so a phone left on a screen is not kept busy. */
const KINDS = {
  hero: { lead: 150, core: 20, seg: 7.2, halo: [[44, 0.16], [68, 0.09], [96, 0.05]], speed: 700, rest: 2.9, begin: 1.85, echo: 0.32, beats: 10 },
  rail: { lead: 100, core: 30, seg: 28.8, halo: [[70, 0.2], [110, 0.11], [150, 0.05]], speed: 3230, rest: 2.8, begin: 1.6, echo: 0.45, beats: 12 },
};

const el = (name, attrs) => {
  const node = document.createElementNS(NS, name);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, String(v));
  return node;
};

/** The zig-zag from `top` down to `end`, and its length. */
const zigzag = (top, end) => ({ d: `M${L1X} ${top}V${STEP_Y}H${L2X}V${end}`, len: STEP_Y - top + (L2X - L1X) + (end - STEP_Y) });

/** Where the line must start and end, in the mark's units, to span the screen the mark sits on, top edge to bottom. */
function span(svg) {
  const screen = svg.closest("section, form");
  const vb = svg.viewBox.baseVal;
  const box = svg.getBoundingClientRect();
  if (!screen || !box.width) return { top: vb.y, end: vb.y + vb.height };
  const per = vb.width / box.width;
  const edges = screen.getBoundingClientRect();
  return {
    top: Math.min(vb.y, Math.floor(vb.y - (box.top - edges.top) * per)),
    end: Math.max(vb.y + vb.height, Math.ceil(vb.y + (edges.bottom - box.top) * per)),
  };
}

/** Stretches the drawn line and its clip to `top`..`end`; the drawing keeps its place [pathLength is 1]. */
function stretch(svg, line, { top, end }) {
  const rects = svg.querySelectorAll("clipPath rect");
  const upper = rects[0];
  const lower = rects[rects.length - 1];
  if (upper && upper !== lower) {
    const bottom = Number(upper.getAttribute("y")) + Number(upper.getAttribute("height"));
    upper.setAttribute("y", String(top - 2));
    upper.setAttribute("height", String(bottom - top + 2));
  }
  if (lower) lower.setAttribute("height", String(end - L2TOP + 2));
  line.setAttribute("d", `M${L1X} ${top}V${STEP_Y}H${L2X}V${end}`);
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

/** The layer the comets live in: an SVG of its own over the line [same box, same units], so a beat repaints only the
 *  comets. Its clock starts when it is first added. */
function pulseLayer(svg) {
  let layer = svg.parentNode.querySelector(":scope > .mk-pulse");
  if (!layer) {
    layer = el("svg", { class: "mk-layer mk-pulse", viewBox: svg.getAttribute("viewBox"), "aria-hidden": "true" });
    svg.after(layer);
  }
  return layer;
}

/** Fits every mark of `root` to its screen and starts the pulse, timed from now; call it whenever that screen shows.
 *  A refit [force false] that moved neither end of the line by more than a few units leaves a running pulse alone. */
export function startPulse(root, { force = true } = {}) {
  if (!root) return;
  const still = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  for (const svg of root.querySelectorAll("svg[data-pulse]")) {
    const kind = KINDS[svg.dataset.pulse];
    const clip = svg.querySelector("clipPath");
    const line = svg.querySelector(".mk-zz");
    if (!kind || !clip || !line) continue;
    const fit = span(svg);
    const moved = Math.abs(fit.end - Number(svg.dataset.end || 0)) + Math.abs(fit.top - Number(svg.dataset.top || 0));
    if (!force && moved < 4) continue;
    svg.dataset.top = String(fit.top);
    svg.dataset.end = String(fit.end);
    stretch(svg, line, fit);
    if (still) continue;
    const layer = pulseLayer(svg);
    const path = zigzag(fit.top - kind.lead, fit.end + 120);
    const travel = path.len / kind.speed;
    const timing = { seg: kind.seg, travel, period: travel + kind.rest, beats: kind.beats, at: layer.getCurrentTime() + kind.begin };
    const box = svg.viewBox.baseVal;
    const first = comet(kind, path, timing, 1, clip.id, box);
    const echo = comet(kind, path, { ...timing, at: timing.at + kind.echo }, 0.65, clip.id, box);
    layer.replaceChildren(first.halo, echo.halo, first.core, echo.core);
  }
}

/** Refits the visible screen's marks whenever a screen changes size: fonts arriving, a step's content, the window,
 *  a phone's toolbar. One refit per frame at most. */
export function watchMarks(screens) {
  if (!("ResizeObserver" in window)) return;
  const changed = new Set();
  let frame = 0;
  const observer = new ResizeObserver((entries) => {
    for (const entry of entries) changed.add(entry.target);
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => {
      for (const screen of changed) if (!screen.hidden) startPulse(screen, { force: false });
      changed.clear();
    });
  });
  for (const screen of screens) if (screen) observer.observe(screen);
}
