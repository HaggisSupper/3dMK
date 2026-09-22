use clap::{Parser, ValueEnum};
use oilwell_core::{convert, ColourPalette, ConversionRequest, InterpolationMethod, OutputFormat};

#[derive(Parser)]
#[command(
    name = "oilwell-converter-cli",
    about = "Offline oilwell table converter"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(clap::Subcommand)]
enum Command {
    Convert(ConvertArgs),
}
#[derive(Parser)]
struct ConvertArgs {
    #[arg(long)]
    survey: String,
    #[arg(long)]
    sections: String,
    #[arg(long)]
    output: String,
    #[arg(long, value_enum, default_value = "glb")]
    format: Format,
    #[arg(long)]
    bha: Option<String>,
    #[arg(long)]
    formations: Option<String>,
    #[arg(long, default_value_t = 36.0)]
    diameter_scale: f64,
    #[arg(long, default_value_t = 75.0)]
    smooth_step_md: f64,
    #[arg(long, value_enum, default_value = "minimum-curvature")]
    interpolation: Interpolation,
    #[arg(long)]
    json: bool,
}
#[derive(Clone, ValueEnum)]
enum Format {
    Glb,
    Obj,
    Ply,
    Stl,
}
#[derive(Clone, ValueEnum)]
enum Interpolation {
    MinimumCurvature,
    LinearDirectionBlend,
}
fn main() {
    let cli = Cli::parse();
    let Command::Convert(args) = cli.command;
    let request = ConversionRequest {
        survey_path: args.survey,
        sections_path: args.sections,
        bha_path: args.bha,
        formation_path: args.formations,
        output_path: args.output,
        output_format: match args.format {
            Format::Glb => OutputFormat::Glb,
            Format::Obj => OutputFormat::Obj,
            Format::Ply => OutputFormat::Ply,
            Format::Stl => OutputFormat::Stl,
        },
        diameter_scale: args.diameter_scale,
        smooth_step_md: args.smooth_step_md,
        interpolation_method: match args.interpolation {
            Interpolation::MinimumCurvature => InterpolationMethod::MinimumCurvature,
            Interpolation::LinearDirectionBlend => InterpolationMethod::LinearDirectionBlend,
        },
        colour_palette: ColourPalette::new(),
    };
    match convert(&request) {
        Ok(result) => {
            if args.json {
                println!("{}", serde_json::to_string(&result).unwrap())
            } else {
                println!("Exported {}", result.output_path)
            }
        }
        Err(error) => {
            if args.json {
                println!("{}", serde_json::to_string(&error).unwrap())
            } else {
                eprintln!("{}", error.message)
            };
            std::process::exit(2)
        }
    }
}
