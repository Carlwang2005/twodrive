import hashlib
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('release_assets', Path(__file__).with_name('verify-release-assets.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseAssets(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.file = Path(self.directory.name, 'package.bin')
        self.file.write_bytes(b'public synthetic package')
        self.release = {'tagName': 'test-v1', 'targetCommitish': 'a' * 40,
                        'isDraft': True, 'isPrerelease': False,
                        'assets': [{'name': self.file.name, 'state': 'uploaded',
                                    'size': self.file.stat().st_size,
                                    'digest': 'sha256:' + hashlib.sha256(self.file.read_bytes()).hexdigest()}]}
        self.calls = []

    def publish(self, files=None):
        return module.check_and_publish(self.release, 'test-v1', 'a' * 40,
                                        files or [self.file], 'stable',
                                        run=lambda *args, **kwargs: self.calls.append(args))

    def test_incomplete_upload_never_reaches_publication(self):
        self.release['assets'] = []
        with self.assertRaises(ValueError):
            self.publish()
        self.assertEqual(self.calls, [])

    def test_state_digest_size_or_target_mismatch_never_publishes(self):
        for key, bad in [('state', 'starter'), ('digest', 'wrong'), ('size', 1)]:
            with self.subTest(key=key):
                saved = self.release['assets'][0][key]
                self.release['assets'][0][key] = bad
                with self.assertRaises(ValueError):
                    self.publish()
                self.release['assets'][0][key] = saved
        self.release['targetCommitish'] = 'b' * 40
        with self.assertRaises(ValueError):
            self.publish()
        self.assertEqual(self.calls, [])

    def test_unexpected_extra_asset_or_wrong_channel_never_publishes(self):
        self.release['assets'].append(dict(self.release['assets'][0]))
        with self.assertRaises(ValueError):
            self.publish()
        self.release['assets'].pop()
        self.release['isPrerelease'] = True
        with self.assertRaises(ValueError):
            self.publish()
        self.assertEqual(self.calls, [])

    def test_valid_draft_publishes_once_and_read_only_check_does_not(self):
        module.check_and_publish(self.release, 'test-v1', 'a' * 40, [self.file])
        self.assertEqual(self.calls, [])
        self.publish()
        self.assertEqual(len(self.calls), 1)
        self.assertIn('--draft=false', self.calls[0][0])
        self.release['isDraft'] = False
        with self.assertRaises(ValueError):
            self.publish()
        self.assertEqual(len(self.calls), 1)
