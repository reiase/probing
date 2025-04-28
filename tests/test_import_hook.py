import sys
import unittest
import os
import tempfile

# Import the module we're testing
from probing.hooks import import_hook


class TestImportHook(unittest.TestCase):
    def setUp(self):
        # Clear any previous registrations for clean testing
        import_hook.register.clear()
        import_hook.triggered.clear()

        # Create a temporary directory for test modules
        self.temp_dir = tempfile.mkdtemp()
        sys.path.insert(0, self.temp_dir)

        # Create a test module
        with open(os.path.join(self.temp_dir, "my_module.py"), "w") as f:
            f.write(
                'TEST_CONSTANT = "Hello from test module"\nprint("This is a test.")\n'
            )

    def tearDown(self):
        # Remove temp directory from path and clean up
        sys.path.remove(self.temp_dir)

        # Remove test modules from sys.modules
        for module_name in list(sys.modules.keys()):
            if module_name.startswith("my_module"):
                del sys.modules[module_name]

    def test_import_hook_new_module(self):
        """Test callback execution when importing a new module."""
        callback_executed = []

        def my_callback(module=None):
            callback_executed.append(True)
            import my_module  # type: ignore

            self.assertEqual(my_module.TEST_CONSTANT, "Hello from test module")

        # Register callback for a module that hasn't been imported yet
        import_hook.register_module_callback("my_module", my_callback)

        # Now import the module - should trigger the callback
        import my_module  # type: ignore

        self.assertTrue(callback_executed)
        self.assertIn("my_module", import_hook.triggered)

    def test_already_imported_module(self):
        """Test callback execution for an already imported module."""
        # First import the module
        import math

        callback_executed = []

        def math_callback(module):
            callback_executed.append(True)
            self.assertTrue(hasattr(module, "pi"))

        # Register callback for already imported module
        import_hook.register_module_callback("math", math_callback)

        # Callback should be executed immediately
        self.assertTrue(callback_executed)
        self.assertIn("math", import_hook.triggered)

    def test_error_in_callback(self):
        """Test error handling in callbacks."""

        def error_callback(module):
            raise ValueError("Test error in callback")

        # Redirect stderr to capture error message
        import io
        from contextlib import redirect_stderr

        f = io.StringIO()
        with redirect_stderr(f):
            # Register callback with intentional error
            import_hook.register_module_callback("os", error_callback)

        # Module should still be marked as triggered despite error
        self.assertIn("os", import_hook.triggered)


if __name__ == "__main__":
    unittest.main()
