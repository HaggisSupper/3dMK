use agentic_cad_backend::{ai_vision, api, cad_engine, point_cloud};
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = "Enterprise Agentic CAD Backend")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a 2D floorplan image to an extruded 3D model
    PdfTo3d {
        #[arg(short, long)]
        image: String,
        /// Extrusion height in meters
        #[arg(short, long, default_value_t = 3.0)]
        height: f64,
        /// Output STEP file path
        #[arg(short, long, default_value = "output.step")]
        output: PathBuf,
    },
    /// Reconstruct a solid surface from a point cloud using Poisson
    PoissonReconstruct {
        #[arg(short, long)]
        input: String,
    },
    /// Start the HTTP API server for the web viewer
    Serve {
        #[arg(short, long, default_value_t = 8181)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::PdfTo3d {
            image,
            height,
            output,
        } => {
            println!("[*] Processing floorplan image: {}", image);
            let coords = ai_vision::extract_floorplan_mistral(image).await?;
            cad_engine::extrude_floorplan(&coords, *height, output)?;
            println!("[+] Successfully generated extruded CAD model.");
        }
        Commands::PoissonReconstruct { input } => {
            println!(
                "[*] Initializing KD-Tree and Poisson Surface Reconstruction on: {}",
                input
            );
            let input_path = std::path::Path::new(input);
            let output_path = std::path::Path::new("output_mesh.ply");
            if point_cloud::pdal_available() {
                point_cloud::run_pdal_pipeline(input_path, output_path)?;
            } else {
                point_cloud::run_implicit_pipeline(input_path, output_path)?;
            }
            println!("[+] Successfully reconstructed point cloud mesh.");
        }
        Commands::Serve { port } => {
            let current_dir = std::env::current_dir()?;
            let output_dir = current_dir.join("output");
            let public_dir = std::env::current_dir()?.join("public");
            std::fs::create_dir_all(&output_dir).ok();
            println!("[*] Starting server on http://localhost:{}", port);
            println!("    Output dir: {}", output_dir.display());
            println!("    Public dir: {}", public_dir.display());
            let router = api::create_router(
                output_dir.clone(),
                public_dir,
                output_dir.join("app-data"),
                *port,
            )?;
            let addr = std::net::SocketAddr::from(([127, 0, 0, 1], *port));
            axum::serve(
                tokio::net::TcpListener::bind(addr).await?,
                router.into_make_service(),
            )
            .await?;
        }
    }
    Ok(())
}
