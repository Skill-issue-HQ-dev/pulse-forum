//! Tests for core::stamp: references get their target's hash in HTML, CSS and JS, outside links stay as they are,
//! a changed file renews the stamps up the chain, and a stamped tree is a fixed point.

use pulse_forum::core::stamp::{hash, stamp, Tree};

fn tree(files: &[(&str, &str)]) -> Tree {
    files.iter().map(|(p, t)| (p.to_string(), t.as_bytes().to_vec())).collect()
}

fn text(t: &Tree, path: &str) -> String {
    String::from_utf8(t[path].clone()).unwrap()
}

#[test]
fn every_kind_of_reference_gets_its_files_hash() {
    let site = tree(&[
        ("index.html", r#"<link href="styles.css"><script type="module" src="app.js"></script><img src="img/logo.svg"><a href="https://example.com/x.css">x</a><a href="mailto:a@b.lv">m</a>"#),
        ("styles.css", r#"@font-face { src: url(fonts/f.woff2) format("woff2"); } .a { background: url("data:image/svg+xml,%3Csvg%3E"); }"#),
        ("app.js", r#"import { A } from "./lib.js"; const later = import("./later.js");"#),
        ("lib.js", "export const A = 1;"),
        ("later.js", "export default 2;"),
        ("img/logo.svg", "<svg/>"),
        ("fonts/f.woff2", "FONT"),
    ]);
    let out = stamp(&site);
    let lib = hash(b"export const A = 1;");
    assert!(text(&out, "app.js").contains(&format!(r#"from "./lib.js?v={lib}""#)));
    assert!(text(&out, "app.js").contains(&format!(r#"import("./later.js?v={}")"#, hash(b"export default 2;"))));
    assert!(text(&out, "styles.css").contains(&format!("url(fonts/f.woff2?v={})", hash(b"FONT"))));
    assert!(text(&out, "styles.css").contains(r#"url("data:image/svg+xml,%3Csvg%3E")"#), "data URIs stay as they are");
    let html = text(&out, "index.html");
    assert!(html.contains(&format!(r#"src="img/logo.svg?v={}""#, hash(b"<svg/>"))));
    assert!(html.contains(&format!(r#"src="app.js?v={}""#, hash(&out["app.js"]))), "the page names the stamped script's own hash");
    assert!(html.contains(&format!(r#"href="styles.css?v={}""#, hash(&out["styles.css"]))));
    assert!(html.contains(r#"href="https://example.com/x.css""#) && html.contains(r#"href="mailto:a@b.lv""#), "outside links stay");
}

#[test]
fn a_stamped_tree_is_a_fixed_point_and_a_change_renews_the_chain() {
    let site = tree(&[("index.html", r#"<script src="app.js"></script>"#), ("app.js", r#"import "./x"; import { B } from "./b.js";"#), ("b.js", "export const B = 1;")]);
    let once = stamp(&site);
    assert_eq!(stamp(&once), once, "stamping again changes nothing");
    let mut changed = once.clone();
    changed.insert("b.js".into(), b"export const B = 2;".to_vec());
    let again = stamp(&changed);
    assert_ne!(text(&again, "app.js"), text(&once, "app.js"), "app.js names the new b.js");
    assert_ne!(text(&again, "index.html"), text(&once, "index.html"), "and the page names the new app.js");
}
