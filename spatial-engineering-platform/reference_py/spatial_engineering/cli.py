from __future__ import annotations
import argparse, json, sys
from pathlib import Path
if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from spatial_engineering.core import Point3, PointCloud, evaluate_distance, fit_plane

def main(argv=None) -> int:
    parser=argparse.ArgumentParser(prog="spatial-reference")
    sub=parser.add_subparsers(dest="command",required=True)
    sub.add_parser("formats")
    for name in ("summarize","fit-plane"):
        p=sub.add_parser(name);p.add_argument("input")
    p=sub.add_parser("decimate");p.add_argument("input");p.add_argument("voxel_size",type=float);p.add_argument("output")
    p=sub.add_parser("measure");p.add_argument("coords",nargs=6,type=float);p.add_argument("--nominal",type=float);p.add_argument("--lower",type=float,default=0.0);p.add_argument("--upper",type=float,default=0.0);p.add_argument("--uncertainty",type=float,default=0.0)
    args=parser.parse_args(argv)
    if args.command=="formats":
        print("native-read: xyz xyzrgb csv txt asc pts ptx ply obj stl off")
        print("adapter-contract: pcd fbx 3mf u3d step iges usd usdz")
    elif args.command=="summarize":
        s=PointCloud.from_path(args.input).summary();print(json.dumps({"count":s.count,"min":s.minimum.__dict__,"max":s.maximum.__dict__,"centroid":s.centroid.__dict__},indent=2))
    elif args.command=="fit-plane":
        f=fit_plane(PointCloud.from_path(args.input));print(json.dumps({"origin":f.origin.__dict__,"normal":f.normal.__dict__,"rms":f.rms,"max_abs":f.max_abs,"confidence":f.confidence},indent=2))
    elif args.command=="decimate":
        c=PointCloud.from_path(args.input);d=c.voxel_decimate(args.voxel_size);Path(args.output).write_text(d.to_xyz(),encoding="utf-8");print(json.dumps({"input":len(c.points),"output":len(d.points),"path":args.output}))
    else:
        a=Point3(*args.coords[:3]);b=Point3(*args.coords[3:]);
        if args.nominal is None: print(json.dumps({"distance":a.distance(b)}))
        else:
            measured,decision=evaluate_distance(a,b,args.nominal,args.lower,args.upper,args.uncertainty);print(json.dumps({"distance":measured,"decision":decision}))
    return 0
if __name__=="__main__": raise SystemExit(main())
