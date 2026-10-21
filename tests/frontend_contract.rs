//! The site talks to one address only: this project's collector behind dbautomatizacijas.com/pulse.
//! Owns the check over frontend/: the address lives in config.js, every fetch goes through it, no other host.
//! Invariant: the entries can only ever be sent to this project's own collector, never another project's.

use std::fs;
use std::path::Path;

const API: &str = "https://dbautomatizacijas.com/pulse/api";
const HOST: &str = "https://dbautomatizacijas.com";
/// The page's own address: link previews (og:url, og:image) name it; nothing is ever sent there.
const SITE: &str = "https://skill-issue-hq-dev.github.io/pulse-forum/";

fn files(ext: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for entry in fs::read_dir(Path::new("frontend")).expect("read frontend/") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) == Some(ext) {
            let text = fs::read_to_string(&path).expect("read file");
            out.push((path.display().to_string(), text));
        }
    }
    assert!(!out.is_empty(), "no .{ext} files in frontend/");
    out
}

fn urls(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for scheme in ["https://", "http://"] {
        for (i, _) in text.match_indices(scheme) {
            let rest = &text[i..];
            let end = rest.find(|c: char| c.is_whitespace() || "\"'`;<>)".contains(c)).unwrap_or(rest.len());
            found.push(rest[..end].to_string());
        }
    }
    found
}

#[test]
fn config_names_the_one_collector_address() {
    let config = fs::read_to_string("frontend/config.js").expect("frontend/config.js");
    assert!(config.contains(&format!("export const API = \"{API}\";")), "config.js must export {API}");
}

#[test]
fn both_pages_take_the_address_from_config() {
    for script in ["frontend/app.js", "frontend/admin.js"] {
        let js = fs::read_to_string(script).expect(script);
        assert!(js.contains("import { API } from \"./config.js\";"), "{script} must import API from config.js");
    }
}

#[test]
fn every_fetch_goes_through_the_api_constant() {
    let mut calls = 0;
    for (name, js) in files("js") {
        for (i, _) in js.match_indices("fetch(") {
            let call = &js[i..(i + 16).min(js.len())];
            assert!(call.starts_with("fetch(`${API}/"), "{name}: a fetch outside API: {call}");
            calls += 1;
        }
    }
    assert!(calls >= 3, "expected the entry, status and admin calls, found {calls}");
}

#[test]
fn no_other_host_appears_in_the_site() {
    let mut texts = files("js");
    texts.extend(files("html"));
    texts.extend(files("css"));
    for (name, text) in texts {
        for url in urls(&text) {
            let ours = url == HOST || url.starts_with(&format!("{HOST}/pulse/"));
            let svg_namespace = url.starts_with("http://www.w3.org/");
            let own_page = url.starts_with(SITE);
            assert!(ours || svg_namespace || own_page, "{name}: unexpected address {url}");
        }
    }
}

#[test]
fn the_pages_allow_connections_to_the_collector_host_only() {
    for page in ["frontend/index.html", "frontend/admin.html"] {
        let html = fs::read_to_string(page).expect(page);
        assert!(html.contains(&format!("connect-src {HOST};")), "{page}: CSP connect-src must be {HOST} only");
    }
}
