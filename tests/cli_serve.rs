//! Tests for src/main.rs through the real binary: `serve` is the only command; it refuses to start without an
//! admin key of 32+ characters, with a bad port or a bad origin list; a real run answers on a loopback socket and
//! its log lines never carry a name, e-mail, phone or story, nor the key.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_pulse-forum");
const KEY: &str = "cli-test-key-cli-test-key-cli-test-key";

fn run(args: &[&str], env: &[(&str, &str)]) -> Output {
    Command::new(BIN).args(args).env_clear().envs(env.iter().copied()).output().expect("run the binary")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

fn refused(out: &Output) -> String {
    assert_eq!(out.status.code(), Some(1), "exit code");
    assert!(out.stdout.is_empty(), "nothing on stdout when refusing: {}", text(&out.stdout));
    text(&out.stderr)
}

#[test]
fn serve_is_the_only_command() {
    for args in [&[][..], &["unknown"], &["serve", "--now"], &["Serve"], &["--help"]] {
        let err = refused(&run(args, &[("PULSE_ADMIN_KEY", KEY)]));
        assert!(err.contains("usage: pulse-forum serve"), "{args:?}: {err}");
    }
}

#[test]
fn no_key_or_a_short_key_means_exit_1_and_a_clear_line() {
    let data = tempfile::tempdir().unwrap();
    let dir = data.path().to_str().unwrap();
    let short = "k".repeat(31);
    let padded = format!("   {}   ", "k".repeat(29));
    for key in [None, Some(""), Some("short"), Some(short.as_str()), Some(padded.as_str())] {
        let mut env = vec![("PULSE_DATA", dir)];
        if let Some(k) = key {
            env.push(("PULSE_ADMIN_KEY", k));
        }
        let err = refused(&run(&["serve"], &env));
        assert!(err.contains("PULSE_ADMIN_KEY") && err.contains("at least 32"), "{key:?}: {err}");
        if let Some(k) = key.filter(|k| k.trim().len() > 4) {
            assert!(!err.contains(k.trim()), "the key must never be printed: {err}");
        }
    }
    assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 0, "a refused start creates nothing");
}

#[test]
fn a_bad_port_or_origin_list_is_refused_before_anything_starts() {
    let data = tempfile::tempdir().unwrap();
    let dir = data.path().to_str().unwrap();
    let base = [("PULSE_ADMIN_KEY", KEY), ("PULSE_DATA", dir)];
    for port in ["abc", "70000", "-1", ""] {
        let mut env = base.to_vec();
        env.push(("PULSE_PORT", port));
        let err = refused(&run(&["serve"], &env));
        assert!(err.contains("PULSE_PORT"), "{port:?}: {err}");
    }
    for origins in ["*", "skill-issue-hq-dev.github.io", "https://a.example/pulse-forum"] {
        let mut env = base.to_vec();
        env.push(("PULSE_ORIGINS", origins));
        let err = refused(&run(&["serve"], &env));
        assert!(err.contains("PULSE_ORIGINS"), "{origins:?}: {err}");
    }
    assert_eq!(std::fs::read_dir(data.path()).unwrap().count(), 0);
}

/// A running service on a free loopback port; killed when dropped.
struct Server {
    child: Child,
    port: u16,
    stdout: Arc<Mutex<Vec<String>>>,
    stderr: Arc<Mutex<Vec<String>>>,
}

fn lines_of(stream: impl Read + Send + 'static) -> Arc<Mutex<Vec<String>>> {
    let lines = Arc::new(Mutex::new(Vec::new()));
    let sink = lines.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(stream).lines().map_while(Result::ok) {
            sink.lock().unwrap().push(line);
        }
    });
    lines
}

impl Server {
    fn start(data: &str, extra_env: &[(&str, &str)]) -> Server {
        let mut cmd = Command::new(BIN);
        cmd.arg("serve").env_clear().env("PULSE_ADMIN_KEY", KEY).env("PULSE_PORT", "0").env("PULSE_DATA", data);
        cmd.envs(extra_env.iter().copied()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = cmd.spawn().expect("start the service");
        let stdout = lines_of(child.stdout.take().unwrap());
        let stderr = lines_of(child.stderr.take().unwrap());
        let deadline = Instant::now() + Duration::from_secs(20);
        let startup = loop {
            if let Some(line) = stdout.lock().unwrap().first().cloned() {
                break line;
            }
            assert!(Instant::now() < deadline && child.try_wait().unwrap().is_none(), "no startup line; stderr: {:?}", stderr.lock().unwrap());
            std::thread::sleep(Duration::from_millis(20));
        };
        let port = startup.split("127.0.0.1:").nth(1).and_then(|r| r.split(',').next()).and_then(|p| p.parse().ok()).unwrap_or_else(|| panic!("no port in {startup:?}"));
        Server { child, port, stdout, stderr }
    }

    /// One HTTP/1.1 request over a plain socket; returns the whole answer text.
    fn http(&self, method: &str, path: &str, headers: &[(&str, &str)], body: &str) -> String {
        let mut s = TcpStream::connect(("127.0.0.1", self.port)).expect("connect");
        s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        let mut req = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n", body.len());
        for (k, v) in headers {
            req.push_str(&format!("{k}: {v}\r\n"));
        }
        s.write_all(format!("{req}\r\n{body}").as_bytes()).unwrap();
        let mut out = Vec::new();
        s.read_to_end(&mut out).unwrap();
        text(&out)
    }

    fn logs(&mut self) -> String {
        let _ = self.child.kill();
        let _ = self.child.wait();
        std::thread::sleep(Duration::from_millis(100));
        let mut all = self.stdout.lock().unwrap().join("\n");
        all.push('\n');
        all.push_str(&self.stderr.lock().unwrap().join("\n"));
        all
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn a_real_run_serves_entries_and_the_admin_and_logs_no_personal_data() {
    let data = tempfile::tempdir().unwrap();
    let dir = data.path().to_str().unwrap();
    let mut server = Server::start(dir, &[]);
    let health = server.http("GET", "/healthz", &[], "");
    assert!(health.starts_with("HTTP/1.1 200") && health.ends_with("{\"ok\":true}"), "{health}");
    assert!(health.to_lowercase().contains("content-security-policy: default-src 'none'; frame-ancestors 'none'"));
    let body = r#"{"token":"0123456789abcdef0123456789abcdef","lang":"lv","category":"trip","story":"ZQXSTORY Satiku jaunu cilvēku pasākumā un runājām par kopīgu projektu.","first":"ZQXFIRST","last":"ZQXLAST","email":"zqx.mail@example.com","phone":"+371 29 999 999","company":"ZQXCOMPANY","consent":true,"share":true,"website":""}"#;
    let posted = server.http("POST", "/api/entry", &[("Content-Type", "text/plain;charset=UTF-8"), ("Origin", "https://skill-issue-hq-dev.github.io")], body);
    assert!(posted.starts_with("HTTP/1.1 200") && posted.ends_with("{\"ok\":true,\"status\":\"new\"}"), "{posted}");
    assert!(posted.to_lowercase().contains("access-control-allow-origin: https://skill-issue-hq-dev.github.io"), "{posted}");
    let denied = server.http("GET", "/api/admin/entries", &[], "");
    assert!(denied.starts_with("HTTP/1.1 401") && denied.ends_with("{\"ok\":false,\"error\":\"key\"}"), "{denied}");
    let auth = format!("Bearer {KEY}");
    let listed = server.http("GET", "/api/admin/entries", &[("Authorization", &auth)], "");
    assert!(listed.starts_with("HTTP/1.1 200") && listed.contains("\"count\":1") && listed.contains("ZQXFIRST"), "{listed}");
    let db = std::fs::metadata(data.path().join("entries.sqlite3")).unwrap();
    assert_eq!(db.permissions().mode() & 0o777, 0o600);

    let logs = server.logs();
    assert!(logs.contains("serve: http://127.0.0.1:") && logs.contains("1 allowed origin(s)"), "{logs}");
    assert!(logs.contains("entry: new") && logs.contains("admin: refused") && logs.contains("admin: entries 1"), "{logs}");
    for secret in ["ZQX", "zqx.mail", "999 999", "Satiku", KEY, "0123456789abcdef"] {
        assert!(!logs.contains(secret), "the log must not carry {secret:?}:\n{logs}");
    }
}

#[test]
fn the_origin_list_and_data_folder_come_from_the_environment() {
    let data = tempfile::tempdir().unwrap();
    let nested = data.path().join("state").join("pulse");
    let mut server = Server::start(nested.to_str().unwrap(), &[("PULSE_ORIGINS", "https://a.example, http://localhost:5173/")]);
    let answer = server.http("GET", "/healthz", &[("Origin", "http://localhost:5173")], "");
    assert!(answer.to_lowercase().contains("access-control-allow-origin: http://localhost:5173"), "{answer}");
    let other = server.http("GET", "/healthz", &[("Origin", "https://skill-issue-hq-dev.github.io")], "");
    assert!(!other.to_lowercase().contains("access-control-allow-origin"), "{other}");
    assert!(nested.join("entries.sqlite3").exists(), "the data folder is created on first start");
    assert!(server.logs().contains("2 allowed origin(s)"));
}
