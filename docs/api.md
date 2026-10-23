# Collector API

The contract between the page (`frontend/`, served by GitHub Pages) and the collector (`src/`, one Rust
service). Public base: `https://dbautomatizacijas.com/pulse/api`. The router in front strips `/pulse`, so the
service itself sees `/api/...`, `/healthz`, `/version`. Times are RFC 3339 UTC (`2026-10-28T10:15:02Z`).

## Service

- `pulse-forum serve` listens on `127.0.0.1:$PULSE_PORT` (default 8820), keeps one SQLite file
  `entries.sqlite3` in `$PULSE_DATA` (default `data`), reads the admin key from `$PULSE_ADMIN_KEY` and refuses to
  start without one of at least 32 characters. Browser origins allowed by CORS: `$PULSE_ORIGINS`, comma
  separated, default `https://skill-issue-hq-dev.github.io`.
- Every answer: `Vary: Origin`, plus `Access-Control-Allow-Origin: <origin>` when the request's `Origin` is on
  the list (never `*`), `Access-Control-Expose-Headers: content-disposition`, and the security headers
  (`content-security-policy: default-src 'none'; frame-ancestors 'none'`, `x-content-type-options: nosniff`,
  `referrer-policy: no-referrer`).
- `OPTIONS` on any path: 204 with `Access-Control-Allow-Methods: GET, POST, OPTIONS`,
  `Access-Control-Allow-Headers: authorization, content-type`, `Access-Control-Max-Age: 600` (allow-origin only
  for a listed origin).
- Body limit 16 KiB (413 above it). Logs: one line per request kind, never a name, e-mail, phone or story.

## Public

`GET /healthz` -> `{"ok":true}` . `GET /version` -> `{"version":"0.1.0"}` . `GET /api/status` -> `{"ok":true,"open":true,"count":12}`
(count = stored entries, the number the start screen shows; nothing else about them is public)

`POST /api/entry`: the body is JSON, sent as `text/plain;charset=UTF-8` so the browser skips the preflight; the
server parses the bytes as JSON whatever the content type.

```json
{
  "token": "32 lowercase hex, made by the phone once per entry",
  "lang": "lv",
  "category": "friendship",
  "story": "empty [the break option: no story, still in the draw] or up to 500 characters after trimming",
  "first": "1-80", "last": "1-80",
  "email": "one @, a dot two characters from the end, max 120",
  "phone": "+ only first, digits, spaces, - ( ) .; 7-15 digits",
  "company": "optional, max 120",
  "consent": true,
  "share": false,
  "website": ""
}
```

- `lang`: `lv` | `en`. `category`: `collaboration`, `business`, `idea`, `event`, `career`, `friendship`,
  `trip`, `other` [the page's order; `new_contact` was retired on 23.10.2026 and is refused].
- Length rules count characters, not bytes (ā is one). Fields are trimmed before every check and stored trimmed.
- `website` is the honeypot: when it is not empty the answer is `{"ok":true,"status":"new"}` and NOTHING is
  stored.
- Answers:
  - 200 `{"ok":true,"status":"new"}`: stored.
  - 200 `{"ok":true,"status":"retry"}`: this token is already stored (a resend after a lost answer); nothing
    changed.
  - 200 `{"ok":true,"status":"taken"}`: this e-mail (trimmed, lower case) is already in the draw under another
    token; the first entry is kept unchanged.
  - 400 `{"ok":false,"error":"json"}`: not JSON or wrong field types.
  - 422 `{"ok":false,"error":CODE}`, CODE one of `token`, `lang`, `category`, `story_long`, `missing` (first,
    last, e-mail or phone empty), `too_long`, `email`, `phone`, `consent`. First failing rule wins, checked in that
    order.
  - 423 `{"ok":false,"error":"closed"}`: entries are closed (checked after the honeypot, before the rules).
    Exception: a valid entry whose token is already stored gets 200 `retry`, so a phone whose answer was lost
    near the close still hears it is in the draw.
  - Same phone: an entry whose phone ends in the same 8 digits as an earlier entry's is stored as `new` like any
    other, but starts excluded with `dup_of` = that earlier entry's id [one person, one chance]; the organisers
    can put it back with `exclude`.
  - 500 `{"ok":false,"error":"server"}`.

## Admin

Every admin call carries `Authorization: Bearer <PULSE_ADMIN_KEY>`, compared in constant time; anything else is
401 `{"ok":false,"error":"key"}`.

- `GET /api/admin/entries` -> 200
  ```json
  {"ok":true,"open":true,"count":12,"eligible":10,
   "entries":[{"id":1,"at":"…","lang":"lv","category":"friendship","story":"…","first":"…","last":"…",
               "email":"…","phone":"…","company":"","share":false,"excluded":false,"won":null,"dup_of":null}],
   "draws":[{"n":1,"at":"…","pool":10,"winner_id":5,"pool_ids":[1,2,4]}]}
  ```
  Entries in submission order; `won` is the draw number that picked this entry, else null; `dup_of` is the
  earlier entry with the same phone, else null; `eligible` = not excluded and never drawn; `pool_ids` = every
  entry the draw drew from.
- `GET /api/admin/export.xlsx` -> 200 workbook,
  `content-type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`,
  `content-disposition: attachment; filename="PULSE izloze YYYY-MM-DD HH-MM.xlsx"` (Riga wall clock, ASCII).
  Sheets: `Stāsti` (one row per entry: Nr., Iesniegts Rīgas laikā, Valoda, Kategorija [Latvian label], Stāsts [empty: the break option],
  Vārds, Uzvārds, E-pasts, Tālrunis, Uzņēmums, Drīkst publicēt stāstu [JĀ/NĒ], Izslēgts no izlozes [JĀ/NĒ],
  Uzvarētājs izlozē Nr., Tas pats tālrunis kā Nr.), `Izlozes` (Nr., Laiks Rīgā, Dalībnieku skaits izlozē,
  Uzvarētājs [entry Nr. + name], Izlozē piedalījās Nr.), `Eksports` (when, how many entries, open or closed). Phone stays text.
- `POST /api/admin/open` `{"open":false}` -> `{"ok":true,"open":false}`.
- `POST /api/admin/exclude` `{"id":5,"excluded":true}` -> `{"ok":true}`; 404 `{"ok":false,"error":"id"}`.
- `POST /api/admin/delete` `{"id":5}` -> `{"ok":true,"deleted":5}` deletes one entry at its owner's request;
  404 `{"ok":false,"error":"id"}`; 409 `{"ok":false,"error":"winner"}` for an entry a draw picked.
- `POST /api/admin/draw` `{}` -> 200 `{"ok":true,"draw":{"n":1,…,"pool_ids":[…]},"winner":{entry}}`;
  409 `{"ok":false,"error":"open"}` while entries are open [nobody still to come is left out]; 409
  `{"ok":false,"error":"empty"}` when nobody is eligible. Uniform pick over the eligible entries with
  operating-system randomness (rejection sampling, no modulo bias); the draw row and the pick are one
  transaction.
- `POST /api/admin/clear` `{"confirm":"DZĒST"}` -> `{"ok":true,"deleted":N}` deletes every entry and draw;
  409 `{"ok":false,"error":"open"}` while entries are open; 422 `{"ok":false,"error":"confirm"}` without the
  exact word.
- Bad JSON on an admin POST: 400 `{"ok":false,"error":"json"}`.

## Labels

| code | LV | EN |
|---|---|---|
| collaboration | Sadarbība | Collaboration |
| business | Uzņēmuma attīstība | Company growth |
| idea | Jauna biznesa ideja | A new business idea |
| event | Pasākuma apmeklējums | An event |
| career | Karjeras pārmaiņas | A career change |
| friendship | Jauna draudzība | A new friendship |
| trip | Ceļojums | A trip |
| other | Cits, vēl labāks stāsts! | Another, even better story! |
