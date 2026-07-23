from __future__ import annotations
from dataclasses import dataclass
from math import floor, isfinite, sqrt
from pathlib import Path
from typing import Iterable

@dataclass(frozen=True)
class Point3:
    x: float
    y: float
    z: float
    def __post_init__(self) -> None:
        if not all(isfinite(v) for v in (self.x, self.y, self.z)):
            raise ValueError("non-finite coordinate")
    def distance(self, other: "Point3") -> float:
        return sqrt((self.x-other.x)**2+(self.y-other.y)**2+(self.z-other.z)**2)

@dataclass(frozen=True)
class PointRecord:
    position: Point3
    rgb: tuple[int, int, int] | None = None

@dataclass(frozen=True)
class CloudSummary:
    count: int
    minimum: Point3
    maximum: Point3
    centroid: Point3

@dataclass(frozen=True)
class PlaneFit:
    origin: Point3
    normal: Point3
    rms: float
    max_abs: float
    confidence: float

class PointCloud:
    def __init__(self, points: Iterable[PointRecord]):
        self.points = tuple(points)
        if not self.points:
            raise ValueError("point cloud is empty")

    @staticmethod
    def from_path(path: str | Path) -> "PointCloud":
        p = Path(path)
        text = p.read_text(encoding="utf-8")
        ext = p.suffix.lower()
        if ext == ".ply": return PointCloud.from_ascii_ply(text)
        if ext == ".pts": return PointCloud.from_pts(text)
        if ext == ".ptx": return PointCloud.from_ptx(text)
        if ext == ".obj": return PointCloud.from_prefixed_vertices(text, "v ")
        if ext == ".stl": return PointCloud.from_prefixed_vertices(text, "vertex ")
        if ext == ".off": return PointCloud.from_off(text)
        if ext in {".xyz", ".xyzrgb", ".csv", ".txt", ".asc"}: return PointCloud.from_xyz_text(text)
        raise ValueError(f"unsupported native point import extension: {ext}")

    @staticmethod
    def from_xyz_text(text: str) -> "PointCloud":
        points: list[PointRecord] = []
        for line_number, raw in enumerate(text.splitlines(), 1):
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            cols = line.replace(",", " ").split()
            if len(cols) < 3:
                raise ValueError(f"line {line_number}: expected x y z")
            try:
                point = Point3(*(float(cols[i]) for i in range(3)))
                rgb = tuple(int(cols[i]) for i in range(3, 6)) if len(cols) >= 6 else None
            except (ValueError, TypeError) as exc:
                raise ValueError(f"line {line_number}: invalid point") from exc
            if rgb is not None and any(v < 0 or v > 255 for v in rgb):
                raise ValueError(f"line {line_number}: RGB outside 0..255")
            points.append(PointRecord(point, rgb))
        return PointCloud(points)


    @staticmethod
    def from_pts(text: str) -> "PointCloud":
        lines = text.splitlines()
        if lines and lines[0].strip().isdigit(): lines = lines[1:]
        return PointCloud.from_xyz_text("\n".join(lines))

    @staticmethod
    def from_ptx(text: str) -> "PointCloud":
        lines = text.splitlines()
        if len(lines) < 10: raise ValueError("PTX header is incomplete")
        columns, rows = int(lines[0]), int(lines[1])
        output=[]
        for raw in lines[10:10+columns*rows]:
            cols=raw.split()
            if len(cols) >= 7: output.append(" ".join(cols[:3]+cols[4:7]))
            else: output.append(" ".join(cols[:3]))
        return PointCloud.from_xyz_text("\n".join(output))

    @staticmethod
    def from_prefixed_vertices(text: str, prefix: str) -> "PointCloud":
        return PointCloud.from_xyz_text("\n".join(line.strip()[len(prefix):] for line in text.splitlines() if line.strip().startswith(prefix)))

    @staticmethod
    def from_off(text: str) -> "PointCloud":
        lines=[line.strip() for line in text.splitlines() if line.strip() and not line.lstrip().startswith("#")]
        if not lines or lines[0] != "OFF": raise ValueError("missing OFF signature")
        count=int(lines[1].split()[0])
        return PointCloud.from_xyz_text("\n".join(lines[2:2+count]))

    @staticmethod
    def from_ascii_ply(text: str) -> "PointCloud":
        lines = text.splitlines()
        if not lines or lines[0] != "ply":
            raise ValueError("missing ply signature")
        if "format ascii 1.0" not in lines:
            raise ValueError("only ASCII PLY is supported")
        count = None
        end = None
        for i, line in enumerate(lines):
            if line.startswith("element vertex "):
                count = int(line.rsplit(" ", 1)[1])
            if line == "end_header":
                end = i + 1
                break
        if count is None or end is None:
            raise ValueError("invalid PLY header")
        cloud = PointCloud.from_xyz_text("\n".join(lines[end:end+count]))
        if len(cloud.points) != count:
            raise ValueError("vertex count mismatch")
        return cloud

    def summary(self) -> CloudSummary:
        xs = [p.position.x for p in self.points]
        ys = [p.position.y for p in self.points]
        zs = [p.position.z for p in self.points]
        n = len(self.points)
        return CloudSummary(n, Point3(min(xs), min(ys), min(zs)), Point3(max(xs), max(ys), max(zs)), Point3(sum(xs)/n, sum(ys)/n, sum(zs)/n))

    def voxel_decimate(self, size: float) -> "PointCloud":
        if not isfinite(size) or size <= 0:
            raise ValueError("voxel size must be positive")
        bins: dict[tuple[int,int,int], list[PointRecord]] = {}
        for p in self.points:
            q = p.position
            bins.setdefault((floor(q.x/size), floor(q.y/size), floor(q.z/size)), []).append(p)
        output: list[PointRecord] = []
        for key in sorted(bins):
            group = bins[key]
            n = len(group)
            position = Point3(sum(p.position.x for p in group)/n, sum(p.position.y for p in group)/n, sum(p.position.z for p in group)/n)
            colors = [p.rgb for p in group if p.rgb is not None]
            rgb = tuple(sum(c[i] for c in colors)//len(colors) for i in range(3)) if len(colors) == n else None
            output.append(PointRecord(position, rgb))
        return PointCloud(output)

    def to_xyz(self) -> str:
        rows = []
        for p in self.points:
            row = f"{p.position.x:.9f} {p.position.y:.9f} {p.position.z:.9f}"
            if p.rgb is not None:
                row += f" {p.rgb[0]} {p.rgb[1]} {p.rgb[2]}"
            rows.append(row)
        return "\n".join(rows) + "\n"

def fit_plane(cloud: PointCloud) -> PlaneFit:
    if len(cloud.points) < 3:
        raise ValueError("plane fit requires at least three points")
    c = cloud.summary().centroid
    xx=xy=xz=yy=yz=zz=0.0
    for p in cloud.points:
        x=p.position.x-c.x; y=p.position.y-c.y; z=p.position.z-c.z
        xx+=x*x; xy+=x*y; xz+=x*z; yy+=y*y; yz+=y*z; zz+=z*z
    dx=yy*zz-yz*yz; dy=xx*zz-xz*xz; dz=xx*yy-xy*xy
    if dx >= dy and dx >= dz: n=(dx, xz*yz-xy*zz, xy*yz-xz*yy)
    elif dy >= dz: n=(xz*yz-xy*zz, dy, xy*xz-yz*xx)
    else: n=(xy*yz-xz*yy, xy*xz-yz*xx, dz)
    length=sqrt(sum(v*v for v in n))
    if length == 0: raise ValueError("degenerate plane")
    normal=Point3(*(v/length for v in n))
    residuals=[abs((p.position.x-c.x)*normal.x+(p.position.y-c.y)*normal.y+(p.position.z-c.z)*normal.z) for p in cloud.points]
    rms=sqrt(sum(r*r for r in residuals)/len(residuals))
    s=cloud.summary(); diagonal=s.minimum.distance(s.maximum) or 1.0
    return PlaneFit(c,normal,rms,max(residuals),max(0.0,min(1.0,1-rms/diagonal)))

def evaluate_distance(a: Point3, b: Point3, nominal: float, lower: float, upper: float, standard_uncertainty: float, coverage_factor: float=2.0) -> tuple[float,str]:
    measured=a.distance(b); expanded=standard_uncertainty*coverage_factor; low=nominal+lower; high=nominal+upper
    if measured-expanded >= low and measured+expanded <= high: decision="pass"
    elif measured+expanded < low or measured-expanded > high: decision="fail"
    else: decision="indeterminate"
    return measured, decision
