// The guest flow: language, category, story, contact, then the answer from the collector.
// Owns screens, the draft kept on the phone, and the outbox that resends until the collector answers.
// Invariant: API is the only address the page calls; "thanks" shows only after the collector stored the entry.

import { CATEGORIES, T } from "./strings.js?v=05228ea6";
import { LIMITS, SERVER_CODES, chars, contactErrors, newToken, storyError } from "./form.js?v=e5e071ba";
import { API } from "./config.js?v=e1e99649";
import { barColour, buzz, confetti, countUp, lvStories, shake } from "./effects.js?v=8ddf25bb";
import { startPulse, watchMarks } from "./pulse.js?v=aee39965";
import { settleAfterKeyboard, toTop } from "./scroll.js?v=4ac86010";

const KEYS = { lang: "pulse-lang", draft: "pulse-draft", outbox: "pulse-outbox" };
const RETRY_MS = [3000, 6000, 12000, 20000, 30000];
const TIMEOUT_MS = 15000;
const LIVE_FROM = 10;
const LIVE_EVERY_MS = 20000;
/** The cover's words have risen by now [motion.css]; the story count joins them, never before them. */
const INTRO_MS = 1300;
const CONTACT_FIELDS = ["first", "last", "email", "phone", "company"];

const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));
const state = { lang: "lv", step: 1, done: null, tries: 0, timer: 0, sending: false };

const store = {
  get(key) { try { return JSON.parse(localStorage.getItem(key) || "null"); } catch { return null; } },
  set(key, value) { try { localStorage.setItem(key, JSON.stringify(value)); } catch { /* private mode: keep going */ } },
  drop(key) { try { localStorage.removeItem(key); } catch { /* nothing to drop */ } },
};

const t = (key) => T[state.lang][key];

/** A string into its element; a *starred* English term inside Latvian text becomes an italic <i lang="en">. */
function setText(el, text) {
  const term = (words) => Object.assign(document.createElement("i"), { lang: "en", textContent: words });
  el.replaceChildren(...text.split("*").flatMap((part, i) => (!part ? [] : i % 2 ? [term(part)] : [part])));
}

function applyLang() {
  document.documentElement.lang = state.lang;
  for (const el of $$("[data-t]")) setText(el, t(el.dataset.t));
  for (const el of $$("[data-t-ph]")) el.placeholder = t(el.dataset.tPh);
  for (const el of $$("[data-t-aria]")) el.setAttribute("aria-label", t(el.dataset.tAria));
  for (const code of CATEGORIES) $(`#cat-${code} .choice-text`).textContent = t("categories")[code];
  $("#rules-list").replaceChildren(...t("rules").map(ruleItem));
  updateCount();
  for (const el of $$(".err")) if (el.dataset.key) el.textContent = t(el.dataset.key);
  if (state.done) renderDone(state.done);
  const k = $("#sending").dataset.key;
  if (k || state.sending) $("#sending").textContent = t(k || "sending");
}

/** One rule as a list item: a line of text, or [its lead, its items] with the items as a list inside it. */
function ruleItem(rule) {
  const item = (text) => Object.assign(document.createElement("li"), { textContent: text });
  if (typeof rule === "string") return item(rule);
  const [lead, ...items] = rule;
  const list = document.createElement("ul");
  list.append(...items.map(item));
  const li = document.createElement("li");
  li.append(lead, list);
  return li;
}

function buildChoices() {
  const list = $("#choices");
  for (const code of CATEGORIES) {
    const label = Object.assign(document.createElement("label"), { className: "choice", id: `cat-${code}` });
    const input = Object.assign(document.createElement("input"), { type: "radio", name: "category", value: code });
    const sq = Object.assign(document.createElement("span"), { className: "sq" });
    sq.setAttribute("aria-hidden", "true");
    label.append(input, sq, Object.assign(document.createElement("span"), { className: "choice-text" }));
    list.append(label);
  }
}

function values() {
  const form = $("#entry");
  const checked = $("input[name=category]:checked", form);
  return {
    category: checked ? checked.value : "",
    story: $("#story").value,
    first: $("#first").value, last: $("#last").value, email: $("#email").value,
    phone: $("#phone").value, company: $("#company").value,
    consent: $("#consent").checked, share: $("#share").checked, pause: $("#pause").checked,
  };
}

function restoreDraft() {
  const d = store.get(KEYS.draft);
  if (!d) return;
  const radio = $(`input[name=category][value="${d.category}"]`);
  if (radio) radio.checked = true;
  for (const f of ["story", "first", "last", "email", "phone", "company"]) if (typeof d[f] === "string") $(`#${f}`).value = d[f];
  $("#consent").checked = !!d.consent;
  $("#share").checked = !!d.share;
  $("#pause").checked = !!d.pause;
  syncPause();
}

/** The break option: no story now, so the story box steps back and step 3 drops the share box [nothing to share]. */
function syncPause() {
  const on = $("#pause").checked;
  $("#step2").classList.toggle("is-pause", on);
  $("#share").closest(".check").hidden = on;
}

function setErr(field, key) {
  const el = $(`#err-${field}`);
  el.dataset.key = key || "";
  el.textContent = key ? t(key) : "";
  const input = field === "category" ? null : $(`#${field}`);
  if (input) input.setAttribute("aria-invalid", key ? "true" : "false");
}

function updateCount() {
  const n = chars($("#story").value.trim());
  const el = $("#story-count");
  el.textContent = t("count")(n);
  el.dataset.state = n > LIMITS.storyMax - 100 ? "near" : n >= LIMITS.storyMin ? "ok" : "short";
}

function show(screen) {
  const fresh = document.body.dataset.screen !== screen;
  $("#start").hidden = screen !== "start";
  $("#entry").hidden = screen !== "flow";
  $("#done").hidden = screen !== "done";
  document.body.dataset.screen = screen;
  barColour(screen);
  if (fresh && screen !== "flow") startPulse($(`#${screen}`));
  toTop();
}

function goStep(n) {
  const way = n >= state.step ? "enter-fwd" : "enter-back";
  state.step = n;
  for (const fs of $$(".step")) {
    const on = Number(fs.dataset.step) === n;
    fs.hidden = !on;
    fs.classList.remove("enter-fwd", "enter-back");
    if (on) {
      void fs.offsetWidth;
      fs.classList.add(way);
    }
  }
  show("flow");
  startPulse($("#entry"));
  holdBack();
  $(`#step${n} .display`).setAttribute("tabindex", "-1");
  $(`#step${n} .display`).focus({ preventScroll: true });
}

/** The phone's Back steps back through the form instead of leaving the page: one history entry while in the flow. */
function holdBack() {
  if (!history.state || !history.state.flow) history.pushState({ flow: true }, "");
}

function onBack() {
  if ($("#entry").hidden) return;
  if (state.sending) return holdBack();
  if (state.step > 1) {
    goStep(state.step - 1);
  } else {
    show("start");
    clearTimeout(state.live);
    checkOpen();
  }
}

/** Marks the step's errors; on a miss, shakes what needs fixing and buzzes once. */
function checkStep(n) {
  const v = values();
  let bad = null;
  if (n === 1) {
    setErr("category", v.category ? null : "errPick");
    bad = v.category ? null : $("#choices");
  } else if (n === 2) {
    const key = v.pause ? null : storyError(v.story);
    setErr("story", key);
    bad = key ? $("#story") : null;
  } else {
    const errs = contactErrors(v);
    for (const f of [...CONTACT_FIELDS, "consent"]) setErr(f, errs[f]);
    const first = [...CONTACT_FIELDS, "consent"].find((f) => errs[f]);
    if (first) $(`#${first}`).focus();
    bad = first ? (first === "consent" ? $("#consent").closest(".check") : $(`#${first}`)) : null;
  }
  if (bad) {
    shake(bad);
    buzz(30);
  }
  return !bad;
}

function renderDone(kind) {
  const title = { thanks: "doneTitle", taken: "takenTitle", closed: "closedTitle" }[kind];
  const text = { thanks: "doneText", taken: "takenText", closed: "closedText" }[kind];
  $("#done-title").textContent = t(title);
  $("#done-text").textContent = t(text);
  $("#done-by").textContent = t("prizeBy");
}

function finish(kind) {
  clearTimeout(state.timer);
  state.sending = false;
  state.pending = null;
  state.done = kind;
  store.drop(KEYS.outbox);
  if (kind !== "closed") store.drop(KEYS.draft);
  renderDone(kind);
  show("done");
  $("#done-title").focus({ preventScroll: true });
  if (kind === "thanks") {
    confetti($("#confetti"));
    buzz([18, 90, 18]);
  }
}

/** The break option sends no story, and with no story there is nothing to share. */
function payload(v, token) {
  return {
    token, lang: state.lang, category: v.category, story: v.pause ? "" : v.story.trim(),
    first: v.first.trim(), last: v.last.trim(), email: v.email.trim(), phone: v.phone.trim(),
    company: v.company.trim(), consent: v.consent, share: v.share && !v.pause, website: $("#website").value,
  };
}

function sendingUi(on, key) {
  state.sending = on;
  // While an answer is on its way its words are fixed: a resend carries the stored copy, so editing would mislead.
  for (const field of $$("#entry input, #entry textarea, #entry [data-back]")) field.disabled = on;
  const button = $("#submit");
  button.disabled = on;
  button.dataset.t = on && key ? "retry" : "submit";
  button.textContent = t(button.dataset.t);
  button.classList.toggle("is-sending", on && !key);
  $("#sending").textContent = key ? t(key) : on ? t("sending") : "";
  $("#sending").dataset.key = key || "";
}

async function post(body) {
  const ctrl = new AbortController();
  const stop = setTimeout(() => ctrl.abort(), TIMEOUT_MS);
  try {
    const res = await fetch(`${API}/entry`, {
      method: "POST", mode: "cors", credentials: "omit", cache: "no-store", signal: ctrl.signal,
      headers: { "Content-Type": "text/plain;charset=UTF-8" }, body: JSON.stringify(body),
    });
    const json = await res.json().catch(() => ({}));
    return { status: res.status, json };
  } finally {
    clearTimeout(stop);
  }
}

function answer(status, json) {
  if (status === 200 && json.ok) return finish(json.status === "taken" ? "taken" : "thanks");
  if (status === 423) return finish("closed");
  if (status === 422 && SERVER_CODES[json.error]) {
    store.drop(KEYS.outbox);
    state.pending = null;
    sendingUi(false);
    const hit = SERVER_CODES[json.error];
    goStep(hit.step);
    return setErr(hit.field, hit.key);
  }
  if (status === 400 || status === 422) {
    store.drop(KEYS.outbox);
    state.pending = null;
    sendingUi(false);
    $("#sending").dataset.key = "errInvalid";
    return ($("#sending").textContent = t("errInvalid"));
  }
  return later("errServer");
}

function later(key) {
  sendingUi(true, key);
  $("#submit").disabled = false;
  const wait = RETRY_MS[Math.min(state.tries, RETRY_MS.length - 1)];
  state.tries += 1;
  clearTimeout(state.timer);
  state.timer = setTimeout(flush, wait);
}

async function flush() {
  const box = store.get(KEYS.outbox) || state.pending;
  if (!box) return;
  clearTimeout(state.timer);
  sendingUi(true);
  try {
    const { status, json } = await post(box);
    answer(status, json);
  } catch {
    later("errOffline");
  }
}

function submit(event) {
  event.preventDefault();
  if (state.sending && !$("#submit").disabled) return flush();
  if (state.sending || !checkStep(3)) return;
  const old = store.get(KEYS.outbox);
  const body = payload(values(), old ? old.token : newToken());
  state.pending = body;
  store.set(KEYS.outbox, body);
  state.tries = 0;
  flush();
}

/** Asks whether entries are open and how many stories are in; repeats while the start screen shows. */
async function checkOpen() {
  try {
    const ctrl = new AbortController();
    setTimeout(() => ctrl.abort(), 8000);
    const res = await fetch(`${API}/status`, { mode: "cors", credentials: "omit", cache: "no-store", signal: ctrl.signal });
    const json = await res.json();
    if (json && json.open === false && !store.get(KEYS.outbox)) return finish("closed");
    if (json && Number.isInteger(json.count)) showLive(json.count);
  } catch {
    /* offline or slow: let the guest start; the send answers for real */
  }
  if (document.body.dataset.screen === "start") state.live = setTimeout(checkOpen, LIVE_EVERY_MS);
}

/** "Jau 128 stāsti / 128 stories so far" on the start screen, once there are enough to be worth showing. */
function showLive(n) {
  const live = $("#live");
  if (n < LIVE_FROM) return;
  $("#live-lv").textContent = `${lvStories(n)} līdz šim`;
  $("#live-en").textContent = `${n === 1 ? "story" : "stories"} so far`;
  setTimeout(() => {
    live.hidden = false;
    countUp($("#live-num"), n);
  }, Math.max(0, INTRO_MS - performance.now()));
}

/** Enter on a contact field moves to the next one; on the last it goes to the consent box. */
function nextOnEnter(e) {
  if (e.key !== "Enter" || e.isComposing) return;
  const i = CONTACT_FIELDS.indexOf(e.target.id);
  if (i < 0) return;
  e.preventDefault();
  const next = CONTACT_FIELDS[i + 1];
  (next ? $(`#${next}`) : $("#consent")).focus();
}

function wire() {
  for (const b of $$("[data-lang]")) b.addEventListener("click", () => { clearTimeout(state.live); setLang(b.dataset.lang); goStep(1); });
  $("#entry").addEventListener("keydown", nextOnEnter);
  $("#lang-switch").addEventListener("click", () => setLang(state.lang === "lv" ? "en" : "lv"));
  for (const b of $$("[data-next]")) b.addEventListener("click", () => { if (checkStep(state.step)) goStep(state.step + 1); });
  for (const b of $$("[data-back]")) b.addEventListener("click", () => goStep(state.step - 1));
  window.addEventListener("popstate", onBack);
  watchMarks([$("#start"), $("#entry"), $("#done")]);
  $("#entry").addEventListener("submit", submit);
  $("#entry").addEventListener("input", (e) => {
    const id = e.target.id;
    if (id === "story") updateCount();
    // Writing takes the break option back; either choice settles the story's error.
    if (id === "story" && $("#pause").checked) $("#pause").checked = false;
    if (id === "story" || id === "pause") syncPause();
    const field = e.target.name === "category" ? "category" : id === "pause" ? "story" : id;
    const err = $(`#err-${field}`);
    if (err && err.dataset.key) setErr(field, null);
    store.set(KEYS.draft, values());
  });
  window.addEventListener("online", () => { if (state.sending) flush(); });
  settleAfterKeyboard();
}

function setLang(lang) {
  state.lang = lang === "en" ? "en" : "lv";
  store.set(KEYS.lang, state.lang);
  applyLang();
}

function init() {
  buildChoices();
  restoreDraft();
  wire();
  setLang(store.get(KEYS.lang) || "en");
  if (store.get(KEYS.outbox)) {
    goStep(3);
    flush();
    return;
  }
  show("start");
  checkOpen();
}

init();
