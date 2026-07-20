#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Context;
use std::path::PathBuf;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

const FRONTEND_HTML: &str = include_str!("../../public/index.html");

fn prepare_app_directories(app: &tauri::AppHandle) -> anyhow::Result<(PathBuf, PathBuf, PathBuf)> {
    let data_dir = app.path().app_data_dir()?;
    let public_dir = data_dir.join("public");
    let output_dir = data_dir.join("output");

    std::fs::create_dir_all(&public_dir)?;
    std::fs::create_dir_all(&output_dir)?;
    std::fs::write(public_dir.join("index.html"), FRONTEND_HTML)?;

    Ok((data_dir, public_dir, output_dir))
}

fn run() -> anyhow::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            let (data_dir, public_dir, output_dir) = prepare_app_directories(app.handle())
                .context("failed to prepare 3DMk application data")?;
            let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
            listener.set_nonblocking(true)?;
            let port = listener.local_addr()?.port();
            let router =
                agentic_cad_backend::api::create_router(output_dir, public_dir, data_dir, port)
                    .context("failed to open 3DMk project storage")?;

            tauri::async_runtime::spawn(async move {
                let listener = tokio::net::TcpListener::from_std(listener)
                    .expect("failed to attach the 3DMk listener to the async runtime");
                if let Err(error) = axum::serve(listener, router.into_make_service()).await {
                    eprintln!("3DMk backend stopped: {error}");
                }
            });

            let url = format!("http://127.0.0.1:{port}").parse()?;
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("3DMk")
                .inner_size(1440.0, 900.0)
                .min_inner_size(900.0, 600.0)
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())?;

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        let log_path = std::env::temp_dir().join("3DMk-startup.log");
        let message = format!("3DMk could not start: {error:#}\n");
        let _ = std::fs::write(&log_path, &message);
        eprintln!("{}See {}", message, log_path.display());
    }
}
