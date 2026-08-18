//! atlas-server binary: hand-parsed CLI, loads compiled data, serves the
//! API (and optionally the published client) over HTTP.
//!
//! ```text
//! atlas-server --data-dir ../data/compiled [--static-dir <path>] [--port 8000]
//! ```

use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use atlas_core::data::AtlasData;

struct Args {
    data_dir: PathBuf,
    static_dir: Option<PathBuf>,
    port: u16,
}

fn parse_args(args: &[String]) -> Result<Args> {
    let mut data_dir: Option<PathBuf> = None;
    let mut static_dir: Option<PathBuf> = None;
    let mut port: u16 = 8000;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--data-dir" => {
                i += 1;
                let v = args.get(i).context("--data-dir requires a value")?;
                data_dir = Some(PathBuf::from(v));
            }
            "--static-dir" => {
                i += 1;
                let v = args.get(i).context("--static-dir requires a value")?;
                static_dir = Some(PathBuf::from(v));
            }
            "--port" => {
                i += 1;
                let v = args.get(i).context("--port requires a value")?;
                port = v.parse().with_context(|| format!("--port value '{v}' is not a valid port number"))?;
            }
            other => bail!("unrecognized argument: {other}"),
        }
        i += 1;
    }

    let data_dir = data_dir.context("--data-dir is required, e.g. --data-dir ../data/compiled")?;
    Ok(Args { data_dir, static_dir, port })
}

#[tokio::main]
async fn main() -> Result<()> {
    // Skip argv[0] (the executable path) before handing off to parse_args,
    // which is written to take a plain argument slice so it stays testable
    // without a process boundary.
    let raw: Vec<String> = env::args().skip(1).collect();
    let args = parse_args(&raw)?;

    let data = AtlasData::load(&args.data_dir)
        .with_context(|| format!("loading compiled data from {}", args.data_dir.display()))?
        .finish();
    let data = Arc::new(data);

    let app = atlas_server::app::build(data, args.static_dir);

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.with_context(|| format!("binding {addr}"))?;
    println!("atlas-server listening on http://{addr}");
    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}
