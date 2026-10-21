//! CLI edge: `pulse-forum serve`, the only command. Reads the environment and starts the service on 127.0.0.1.
//! Owns argument and environment parsing and the exit code; logic lives in core, I/O in io, HTTP in web.
//! Invariant: no admin key of at least 32 characters means exit 1 with one clear line on stderr (the key is never printed).

use pulse_forum::web::{self, Settings};
use pulse_forum::config;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: pulse-forum serve";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [command] if command == "serve" => run_serve(),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run_serve() -> Result<(), String> {
    let origins = std::env::var(config::ORIGINS_ENV).unwrap_or_default();
    let mut origins = web::parse_origins(&origins);
    if origins.is_empty() {
        origins = web::parse_origins(config::DEFAULT_ORIGINS);
    }
    let settings = Settings {
        port: env_port()?,
        data: PathBuf::from(std::env::var(config::DATA_ENV).unwrap_or_else(|_| config::DEFAULT_DATA_DIR.to_string())),
        admin_key: std::env::var(config::ADMIN_KEY_ENV).unwrap_or_default().trim().to_string(),
        origins,
    };
    let runtime = tokio::runtime::Runtime::new().map_err(|e| format!("serve: runtime: {e}"))?;
    runtime.block_on(web::serve(&settings))
}

fn env_port() -> Result<u16, String> {
    match std::env::var(config::PORT_ENV) {
        Ok(v) => v.parse().map_err(|e| format!("serve: {}={v:?}: {e}", config::PORT_ENV)),
        Err(_) => Ok(config::DEFAULT_PORT),
    }
}
