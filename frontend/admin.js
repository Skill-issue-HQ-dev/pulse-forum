// The organisers' page: reads every entry with the key from the private link, and runs the draw.
// Owns the key handling (link fragment -> this tab only), rendering and the admin calls.
// Invariant: guest text is only ever set as textContent, and the key never leaves this tab except to API.

import { API } from "./config.js";
import { CATEGORIES, T } from "./strings.js";

const KEY_STORE = "pulse-admin-key";
const REFRESH_MS = 20000;
const LABELS = T.lv.categories;
const $ = (sel) => document.querySelector(sel);
const state = { key: "", data: null, busy: false, timer: 0 };

function readKey() {
  const fromLink = decodeURIComponent(location.hash.replace(/^#(k=)?/, ""));
  if (fromLink) {
    try { sessionStorage.setItem(KEY_STORE, fromLink); } catch { /* keep it in memory only */ }
    history.replaceState(null, "", location.pathname);
    return fromLink;
  }
  try { return sessionStorage.getItem(KEY_STORE) || ""; } catch { return ""; }
}

function say(text) { $("#msg").textContent = text; }

const riga = (iso) => new Date(iso).toLocaleString("lv-LV", { timeZone: "Europe/Riga", day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });

async function call(path, body) {
  const init = { mode: "cors", credentials: "omit", cache: "no-store", headers: { Authorization: `Bearer ${state.key}` } };
  if (body !== undefined) Object.assign(init, { method: "POST", body: JSON.stringify(body) }, { headers: { ...init.headers, "Content-Type": "application/json" } });
  const res = await fetch(`${API}/admin/${path}`, init);
  if (res.status === 401) throw new Error("Atslēga neder. Atver saiti, ko saņēmi, vēlreiz.");
  return res;
}

async function load(keepMessage = false) {
  try {
    const res = await call("entries");
    const json = await res.json();
    if (!json.ok) throw new Error("Neizdevās nolasīt ierakstus.");
    state.data = json;
    render(json);
    $("#updated").textContent = `Atjaunots ${new Date().toLocaleTimeString("lv-LV", { timeZone: "Europe/Riga" })}`;
    if (!keepMessage) say("");
  } catch (e) {
    say(e instanceof TypeError ? "Nav savienojuma ar serveri. Mēģinām vēlreiz pēc brīža." : e.message);
  }
}

function el(tag, cls, text) {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = text;
  return node;
}

function render(d) {
  for (const id of ["overview", "draws-panel", "entries-panel", "danger"]) $(`#${id}`).hidden = false;
  $("#count").textContent = d.count;
  $("#eligible").textContent = d.eligible;
  $("#state").textContent = d.open ? "Atvērta" : "Slēgta";
  $("#toggle").textContent = d.open ? "Slēgt pieteikšanos" : "Atvērt pieteikšanos";
  $("#clear").disabled = d.open;
  $("#draw").disabled = d.open || d.eligible === 0;
  $("#draw-note").hidden = !d.open;
  renderCats(d.entries);
  renderDraws(d);
  renderEntries(d.entries);
  const last = d.draws[d.draws.length - 1];
  if (last) showWinner(d.entries.find((e) => e.id === last.winner_id), last);
}

function renderCats(entries) {
  const counts = Object.fromEntries(CATEGORIES.map((c) => [c, 0]));
  for (const e of entries) counts[e.category] = (counts[e.category] || 0) + 1;
  $("#cats").replaceChildren(...CATEGORIES.map((c) => el("li", "", `${LABELS[c]}: ${counts[c]}`)));
}

function renderDraws(d) {
  const byId = new Map(d.entries.map((e) => [e.id, e]));
  $("#draws").replaceChildren(...d.draws.map((dr) => {
    const w = byId.get(dr.winner_id);
    const who = w ? `${w.first} ${w.last}, ${w.phone}` : `ieraksts Nr. ${dr.winner_id}`;
    return el("li", "", `Izloze Nr. ${dr.n}, ${riga(dr.at)}, no ${dr.pool} dalībniekiem: ${who}`);
  }));
  $("#draws-panel").hidden = d.draws.length === 0;
}

function entryRow(e) {
  const li = el("li", e.excluded ? "entry is-out" : e.won ? "entry is-won" : "entry");
  const head = el("p", "entry-head");
  head.append(el("span", "entry-no", `Nr. ${e.id}`), el("span", "", riga(e.at)), el("span", "tag", LABELS[e.category] || e.category), el("span", "tag", e.lang.toUpperCase()));
  if (e.won) head.append(el("span", "tag tag--won", `Uzvarētājs, izloze Nr. ${e.won}`));
  if (e.dup_of) head.append(el("span", "tag tag--twin", `Tas pats tālrunis kā Nr. ${e.dup_of}`));
  const name = el("p", "entry-name", `${e.first} ${e.last}${e.company ? `, ${e.company}` : ""}`);
  const contact = el("p", "entry-contact", `${e.phone} / ${e.email}`);
  const story = el("p", "entry-story", e.story);
  const flags = el("p", "entry-flags", e.share ? "Drīkst publicēt stāstu ar vārdu" : "Stāstu publicēt nedrīkst");
  const toggle = el("button", "btn btn--line btn--small", e.excluded ? "Iekļaut izlozē" : "Izslēgt no izlozes");
  toggle.type = "button";
  toggle.addEventListener("click", () => exclude(e.id, !e.excluded));
  const actions = el("p", "entry-actions");
  actions.append(toggle);
  if (!e.won) {
    const drop = el("button", "btn btn--line btn--small", "Dzēst");
    drop.type = "button";
    drop.addEventListener("click", () => remove(e.id));
    actions.append(drop);
  }
  li.append(head, name, contact, story, flags, actions);
  return li;
}

function renderEntries(entries) {
  $("#entries").replaceChildren(...entries.slice().reverse().map(entryRow));
}

function showWinner(w, dr) {
  if (!w) return;
  $("#winner").hidden = false;
  $("#winner-label").textContent = `Uzvarētājs, izloze Nr. ${dr.n}`;
  $("#winner-name").textContent = `${w.first} ${w.last}`;
  Object.assign($("#winner-phone"), { textContent: w.phone, href: `tel:${w.phone.replace(/[^+\d]/g, "")}` });
  Object.assign($("#winner-email"), { textContent: w.email, href: `mailto:${encodeURIComponent(w.email)}` });
  $("#winner-company").textContent = w.company || "";
  $("#winner-story").textContent = w.story;
  $("#winner-meta").textContent = `${LABELS[w.category] || w.category}. Izlozēts ${riga(dr.at)} no ${dr.pool} dalībniekiem.`;
}

async function act(fn) {
  if (state.busy) return;
  state.busy = true;
  try { await fn(); } catch (e) { say(e instanceof TypeError ? "Nav savienojuma ar serveri." : e.message); }
  state.busy = false;
  await load(true);
}

function draw() {
  const n = state.data ? state.data.eligible : 0;
  if (!confirm(`Izlozēt uzvarētāju no ${n} dalībniekiem?`)) return;
  return act(async () => {
    const res = await call("draw", {});
    const json = await res.json();
    if (res.status === 409) throw new Error(json.error === "open" ? "Vispirms slēdz pieteikšanos." : "Izlozē neviens nepiedalās.");
    if (!json.ok) throw new Error("Izloze neizdevās.");
    say(`Uzvarētājs izlozēts: ${json.winner.first} ${json.winner.last}.`);
  });
}

function toggleOpen() {
  const open = !(state.data && state.data.open);
  if (!open && !confirm("Slēgt pieteikšanos? Jaunus stāstus vairs nepieņems.")) return;
  return act(async () => { await call("open", { open }); });
}

function exclude(id, excluded) {
  return act(async () => {
    const res = await call("exclude", { id, excluded });
    if (!res.ok) throw new Error("Neizdevās mainīt ierakstu.");
  });
}

function remove(id) {
  if (!confirm(`Dzēst stāstu Nr. ${id}? To vairs nevarēs atjaunot.`)) return;
  return act(async () => {
    const res = await call("delete", { id });
    if (res.status === 409) throw new Error("Uzvarētāja ierakstu dzēst nevar.");
    if (!res.ok) throw new Error("Neizdevās dzēst ierakstu.");
    say(`Stāsts Nr. ${id} izdzēsts.`);
  });
}

async function exportXlsx() {
  try {
    const res = await call("export.xlsx");
    if (!res.ok) throw new Error("Excel neizdevās.");
    const name = /filename="([^"]+)"/.exec(res.headers.get("content-disposition") || "")?.[1] || "PULSE izloze.xlsx";
    const url = URL.createObjectURL(await res.blob());
    const a = Object.assign(document.createElement("a"), { href: url, download: name });
    document.body.append(a);
    a.click();
    a.remove();
    setTimeout(() => URL.revokeObjectURL(url), 10000);
  } catch (e) {
    say(e instanceof TypeError ? "Nav savienojuma ar serveri." : e.message);
  }
}

function clearAll() {
  const typed = prompt("Lai izdzēstu visus stāstus un izlozes, ieraksti: DZĒST");
  if (typed === null) return;
  return act(async () => {
    const res = await call("clear", { confirm: typed.trim() });
    const json = await res.json();
    if (res.status === 409) throw new Error("Vispirms slēdz pieteikšanos.");
    if (!json.ok) throw new Error("Nedzēsa: jāieraksta tieši DZĒST.");
    $("#winner").hidden = true;
    say(`Izdzēsti ${json.deleted} ieraksti.`);
  });
}

function init() {
  state.key = readKey();
  if (!state.key) return say("Nav atslēgas. Atver saiti, ko saņēmi.");
  $("#draw").addEventListener("click", draw);
  $("#toggle").addEventListener("click", toggleOpen);
  $("#export").addEventListener("click", exportXlsx);
  $("#clear").addEventListener("click", clearAll);
  load();
  state.timer = setInterval(() => { if (!state.busy && !document.hidden) load(); }, REFRESH_MS);
}

init();
