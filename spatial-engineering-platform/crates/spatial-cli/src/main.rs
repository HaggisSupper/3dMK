use format_contracts::CAPABILITIES;
use geometry_fit::fit_plane;
use metrology::DistanceMeasurement;
use pointcloud_core::PointCloud;
use format_io::{read_point_cloud, read_mesh, write_point_cloud, write_mesh};
use spatial_types::{ArtifactId, Point3, Provenance, StandardUncertainty};
use std::{env, fs, path::Path};

fn main(){if let Err(e)=run(){eprintln!("error: {e}");std::process::exit(2)}}
fn run()->Result<(),Box<dyn std::error::Error>>{let args:Vec<String>=env::args().collect();match args.get(1).map(String::as_str){
Some("summarize")=>{let c=load(&args,2)?;let s=c.summary()?;println!("count={} min={:?} max={:?} centroid={:?}",s.count,s.min,s.max,s.centroid);}
Some("decimate")=>{let c=load(&args,2)?;let size: f64=args.get(3).ok_or("missing voxel size")?.parse()?;let out=args.get(4).ok_or("missing output path")?;let d=c.voxel_decimate(size)?;fs::write(out,d.to_xyz())?;println!("input={} output={} path={out}",c.points.len(),d.points.len());}
Some("fit-plane")=>{let c=load(&args,2)?;let f=fit_plane(&c)?;println!("origin={:?} normal={:?} rms={:.9} max_abs={:.9} confidence={:.6}",f.origin,f.normal,f.quality.rms,f.quality.max_abs,f.quality.confidence.value());}
Some("formats")=>{for c in CAPABILITIES{println!(".{:<7} {:?} read={:?} write={:?} -- {}",c.extension,c.kind,c.read,c.write,c.notes);}}
Some("convert-cloud")=>{let input=args.get(2).ok_or("missing input path")?;let output=args.get(3).ok_or("missing output path")?;let cloud=read_point_cloud(Path::new(input))?;write_point_cloud(Path::new(output),&cloud)?;println!("points={} output={output}",cloud.points.len());}
Some("convert-mesh")=>{let input=args.get(2).ok_or("missing input path")?;let output=args.get(3).ok_or("missing output path")?;let mesh=read_mesh(Path::new(input))?;write_mesh(Path::new(output),&mesh)?;println!("vertices={} triangles={} output={output}",mesh.vertices.len(),mesh.triangles.len());}
Some("measure")=>{if args.len()!=9{return Err("usage: measure x1 y1 z1 x2 y2 z2".into());}let v:Vec<f64>=args[2..].iter().map(|x|x.parse()).collect::<Result<_,_>>()?;let m=DistanceMeasurement::evaluate(Point3::new(v[0],v[1],v[2])?,Point3::new(v[3],v[4],v[5])?,None,None,None,StandardUncertainty::new(0.)?,2.,Provenance{method:"cli-point-distance".into(),implementation_version:env!("CARGO_PKG_VERSION").into(),source_artifacts:vec![ArtifactId("manual".into())],parameters:"{}".into()})?;println!("distance={:.9}",m.measured);}
_=>print_help(),}Ok(())}
fn load(args:&[String],index:usize)->Result<PointCloud,Box<dyn std::error::Error>>{let p=args.get(index).ok_or("missing input path")?;Ok(read_point_cloud(Path::new(p))?)}
fn print_help(){println!("spatial-cli commands:\n  formats\n  convert-cloud <input> <output>\n  convert-mesh <input> <output>\n  summarize <supported-point-cloud>\n  decimate <input> <voxel-size> <output.xyz>\n  fit-plane <input>\n  measure <x1> <y1> <z1> <x2> <y2> <z2>");}
