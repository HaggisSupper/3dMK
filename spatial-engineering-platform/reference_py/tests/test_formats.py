import tempfile, unittest
from pathlib import Path
from spatial_engineering.core import PointCloud

class FormatTests(unittest.TestCase):
    def test_pts_count_header(self):
        self.assertEqual(len(PointCloud.from_pts("2\n0 0 0\n1 2 3\n").points),2)
    def test_obj_vertices(self):
        self.assertEqual(len(PointCloud.from_prefixed_vertices("v 0 0 0\nv 1 2 3\nf 1 2 2\n","v ").points),2)
    def test_off_vertices(self):
        self.assertEqual(len(PointCloud.from_off("OFF\n3 1 0\n0 0 0\n1 0 0\n0 1 0\n3 0 1 2\n").points),3)
    def test_path_rejects_unknown(self):
        with tempfile.TemporaryDirectory() as d:
            q=Path(d)/"a.foo";q.write_text("0 0 0")
            with self.assertRaises(ValueError): PointCloud.from_path(q)
if __name__=="__main__": unittest.main()
