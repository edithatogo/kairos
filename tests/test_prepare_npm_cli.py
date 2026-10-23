"""The pinned npm artifact must not vary with Python's compression defaults."""
import gzip
import io
import unittest

from test_bootstrap_postcss_pin import PREPARER, npm_archive, package_members


class NpmCompressionTests(unittest.TestCase):
    def test_repack_matches_explicit_level_nine_and_is_repeatable(self):
        source = npm_archive(package_members())
        actual = PREPARER.repack(source)
        output = io.BytesIO()
        with gzip.GzipFile(fileobj=output, mode='wb', mtime=0, compresslevel=9) as stream:
            stream.write(gzip.decompress(actual))
        self.assertEqual(actual, output.getvalue())
        self.assertEqual(actual, PREPARER.repack(source))


if __name__ == '__main__':
    unittest.main()
