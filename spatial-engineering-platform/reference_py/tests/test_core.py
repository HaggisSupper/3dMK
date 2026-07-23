import sys, unittest
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from spatial_engineering.core import Point3, PointCloud, evaluate_distance, fit_plane
class CoreTests(unittest.TestCase):
    def test_summary(self):
        s=PointCloud.from_xyz_text("0 0 0\n2 4 6\n").summary();self.assertEqual(s.centroid,Point3(1,2,3))
    def test_voxel_decimation(self):
        d=PointCloud.from_xyz_text("0 0 0\n0.1 0 0\n2 0 0\n").voxel_decimate(1);self.assertEqual(len(d.points),2);self.assertAlmostEqual(d.points[0].position.x,.05)
    def test_plane_fit(self):
        f=fit_plane(PointCloud.from_xyz_text("0 0 2\n1 0 2\n0 1 2\n1 1 2\n"));self.assertGreater(abs(f.normal.z),.999999);self.assertLess(f.rms,1e-12)
    def test_uncertainty_can_be_indeterminate(self):
        _,decision=evaluate_distance(Point3(0,0,0),Point3(10.09,0,0),10,-.1,.1,.02);self.assertEqual(decision,"indeterminate")
    def test_malformed_point_rejected(self):
        with self.assertRaises(ValueError):PointCloud.from_xyz_text("0 1\n")
if __name__=="__main__":unittest.main()
