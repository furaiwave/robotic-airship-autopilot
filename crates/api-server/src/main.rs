//! `airship-api [--host 127.0.0.1] [--port 8080] [--static-dir apps/web/dist]`

use std::net::SocketAddr;
use std::path::PathBuf;

use tracing_subscriber::EnvFilter;

fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tower_http=info")),
        )
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let host = arg(&args, "--host")
        .or_else(|| std::env::var("AIRSHIP_HOST").ok())
        .unwrap_or_else(|| "127.0.0.1".into());
    let port: u16 = arg(&args, "--port")
        .or_else(|| std::env::var("AIRSHIP_PORT").ok())
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let static_dir = arg(&args, "--static-dir")
        .or_else(|| std::env::var("AIRSHIP_STATIC_DIR").ok())
        .map(PathBuf::from)
        .filter(|p| p.join("index.html").exists());

    let addr: SocketAddr = match format!("{host}:{port}").parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("invalid address {host}:{port}: {e}");
            std::process::exit(2);
        }
    };
    if let Some(d) = &static_dir {
        tracing::info!(dir = %d.display(), "serving the web application");
    }
    let app = api_server::router(api_server::app_state(), static_dir);
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot bind {addr}: {e}");
            std::process::exit(1);
        }
    };
    tracing::info!(%addr, "airship simulator API listening");
    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("shutting down");
    };
    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
    {
        eprintln!("server error: {e}");
        std::process::exit(1);
    }
}
