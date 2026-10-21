// The pulse along the PULSE logo's line: a light signal flowing down, right along the L's foot and down again,
// twice in a row like a heartbeat [ba-dum], then a rest. Builds the comets into every svg[data-pulse] of a screen.
// Invariant: decoration only, starting after the line has drawn itself; nothing at all under reduced motion.

const NS = "http://www.w3.org/2000/svg";
const ORANGE = "#E07D14";
/** The tail's fade behind the head, step by step. */
const TAIL = Array.from({ length: 12 }, (_, k) => (1 - k / 12) ** 2);
/** Layer j covers the first j+1 steps; these opacities stack up to TAIL without seams between the steps. */
const LAYERS = TAIL.map((a, k) => (k === TAIL.length - 1 ? a : 1 - (1 - a) / (1 - TAIL[k + 1])));

/** Path in the mark's own units [starts above the mark, ends below it], its length, and how each mark beats. */
const KINDS = {
  hero: {
    d: "M343.46 -150V563.39H380.83V1000", len: 1187.37, core: 20, seg: 12,
    halo: [[44, 0.16], [68, 0.09], [96, 0.05]], travel: 1.7, period: 4.6, begin: 1.25, echo: 0.32,
  },
  rail: {
    d: "M343.46 100V563.39H380.83V9100", len: 9037.37, core: 30, seg: 48,
    halo: [[70, 0.2], [110, 0.11], [150, 0.05]], travel: 2.8, period: 5.6, begin: 1.6, echo: 0.45,
  },
};

const el = (name, attrs) => {
  const node = document.createElementNS(NS, name);
  for (const [k, v] of Object.entries(attrs)) node.setAttribute(k, String(v));
  return node;
};

/** One dash whose front runs from `ahead` to past the path's end, eased, then rests until the next beat. */
function moving(kind, at, { length, width, opacity, ahead = 0, cap = "butt" }) {
  const from = length - ahead;
  const to = from - (kind.len + (TAIL.length + 2) * kind.seg);
  const path = el("path", {
    d: kind.d, fill: "none", stroke: ORANGE, "stroke-width": width, opacity, "stroke-linecap": cap,
    "stroke-dasharray": `${length} 30000`, "stroke-dashoffset": from,
  });
  path.append(el("animate", {
    attributeName: "stroke-dashoffset", values: `${from};${to};${to}`,
    keyTimes: `0;${(kind.travel / kind.period).toFixed(4)};1`, calcMode: "spline",
    keySplines: "0.45 0 0.25 1;0 0 1 1", dur: `${kind.period}s`, begin: `${at.toFixed(2)}s`, repeatCount: "indefinite",
  }));
  return path;
}

/** A comet: a soft glow under the mark, and inside the line a round head with a fading tail. The fill-less rect
 *  over the whole mark keeps WebKit from clipping the comet to its path's narrow box. */
function comet(kind, at, strength, clipId, box) {
  const halo = el("g", { class: "mk-comet", opacity: strength });
  for (const [width, opacity] of kind.halo) {
    halo.append(moving(kind, at, { length: kind.seg * 2, width, opacity, ahead: kind.seg * 0.4, cap: "round" }));
  }
  const core = el("g", { class: "mk-comet", opacity: strength, "clip-path": `url(#${clipId})` });
  core.append(el("rect", { x: box.x, y: box.y, width: box.width, height: box.height, fill: "none" }));
  LAYERS.forEach((opacity, j) => core.append(moving(kind, at, { length: kind.seg * (j + 1), width: kind.core, opacity })));
  core.append(moving(kind, at, { length: 0.01, width: kind.core, opacity: 1, cap: "round" }));
  return { halo, core };
}

/** Starts the pulse in every mark of `root`, timed from now; call it each time that screen shows. */
export function startPulse(root) {
  if (!root || window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;
  for (const svg of root.querySelectorAll("svg[data-pulse]")) {
    const kind = KINDS[svg.dataset.pulse];
    const clip = svg.querySelector("clipPath");
    const letters = svg.querySelector(".mk-letters");
    const line = svg.querySelector(".mk-zz");
    if (!kind || !clip || !letters || !line) continue;
    for (const old of svg.querySelectorAll(".mk-comet")) old.remove();
    const at = svg.getCurrentTime() + kind.begin;
    const box = svg.viewBox.baseVal;
    const first = comet(kind, at, 1, clip.id, box);
    const echo = comet(kind, at + kind.echo, 0.65, clip.id, box);
    letters.before(first.halo, echo.halo);
    line.parentNode.after(first.core, echo.core);
  }
}
