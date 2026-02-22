use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use rein::parser::parse;
use rein::server::{AppState, serve};

/// Run the Rein API server.
pub fn run_serve(file: &Path, host: &str, port: u16) -> i32 {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: could not read {}: {e}", file.display());
            return 1;
        }
    };

    let rein_file = match parse(&source) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("parse error: {e}");
            return 1;
        }
    };

    let state = Arc::new(AppState {
        rein_file,
        audit_log: None,
    });

    let addr: SocketAddr = match format!("{host}:{port}").parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: invalid address {host}:{port}: {e}");
            return 1;
        }
    };

    eprintln!("rein serve listening on http://{addr}");

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    if let Err(e) = rt.block_on(serve(state, addr)) {
        eprintln!("server error: {e}");
        return 1;
    }

    0
}
