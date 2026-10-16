use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use vwm_implicit::{OrientedPointSet, PoissonConfig, PoissonReconstructor};
use vwm_io::load_scene;

fn main() -> Result<()> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() < 2 || arguments.len() > 4 {
        bail!("usage: reconstruct_scene <input> <output.obj> [sample_stride=1] [max_depth=8]");
    }

    let input_path = PathBuf::from(&arguments[0]);
    let output_path = PathBuf::from(&arguments[1]);
    let sample_stride = parse_positive(arguments.get(2), 1, "sample stride")?;
    let max_depth = parse_positive(arguments.get(3), 8, "maximum depth")?;

    let load_started = Instant::now();
    let scene = load_scene(&input_path)
        .with_context(|| format!("failed to load {}", input_path.display()))?;
    let loaded_points = scene.vertices.len();
    let oriented = OrientedPointSet::from_canonical_scene(&scene)?;
    let sampled = sample_oriented_points(&oriented, sample_stride)?;
    let load_elapsed = load_started.elapsed();
    eprintln!(
        "loaded {loaded_points} points and selected {} in {} ms",
        sampled.points.len(),
        load_elapsed.as_millis()
    );

    let reconstruction_started = Instant::now();
    eprintln!("starting Poisson reconstruction at depth {max_depth}");
    let field = PoissonReconstructor.reconstruct(
        &sampled,
        PoissonConfig {
            density_estimation_depth: max_depth.min(5),
            max_depth,
            ..PoissonConfig::default()
        },
    )?;
    let mesh = field.reconstruct_mesh()?;
    let reconstruction_elapsed = reconstruction_started.elapsed();
    write_obj(
        &output_path,
        &mesh.positions,
        &mesh.normals,
        &mesh.triangles,
    )?;

    println!(
        "input_points={loaded_points} sampled_points={} output_vertices={} output_triangles={} load_ms={} reconstruction_ms={} stride={sample_stride} depth={max_depth}",
        sampled.points.len(),
        mesh.positions.len(),
        mesh.triangles.len(),
        load_elapsed.as_millis(),
        reconstruction_elapsed.as_millis(),
    );
    Ok(())
}

fn parse_positive(value: Option<&String>, default: usize, name: &str) -> Result<usize> {
    let parsed = value.map_or(Ok(default), |value| {
        value
            .parse::<usize>()
            .with_context(|| format!("invalid {name}: {value}"))
    })?;
    if parsed == 0 {
        bail!("{name} must be positive");
    }
    Ok(parsed)
}

fn sample_oriented_points(input: &OrientedPointSet, stride: usize) -> Result<OrientedPointSet> {
    if stride == 1 {
        return Ok(input.clone());
    }
    let selected = (0..input.points.len()).step_by(stride).collect::<Vec<_>>();
    let mut sampled = OrientedPointSet::new(
        selected.iter().map(|index| input.points[*index]).collect(),
        selected.iter().map(|index| input.normals[*index]).collect(),
    )?;
    sampled.confidence = input
        .confidence
        .as_ref()
        .map(|values| selected.iter().map(|index| values[*index]).collect());
    sampled.source_ids = input
        .source_ids
        .as_ref()
        .map(|values| selected.iter().map(|index| values[*index]).collect());
    sampled.validate()?;
    Ok(sampled)
}

fn write_obj(
    path: &Path,
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    triangles: &[[u32; 3]],
) -> Result<()> {
    let file = File::create(path)
        .with_context(|| format!("failed to create output OBJ at {}", path.display()))?;
    let mut writer = BufWriter::new(file);
    for position in positions {
        writeln!(writer, "v {} {} {}", position[0], position[1], position[2])?;
    }
    for normal in normals {
        writeln!(writer, "vn {} {} {}", normal[0], normal[1], normal[2])?;
    }
    for triangle in triangles {
        let a = triangle[0] + 1;
        let b = triangle[1] + 1;
        let c = triangle[2] + 1;
        writeln!(writer, "f {a}//{a} {b}//{b} {c}//{c}")?;
    }
    writer.flush()?;
    Ok(())
}
