import importlib.abc
import importlib.util
import sys

from probing.ext.torch import init as torch_init

# Mapping from module names to callback functions
register = {
    "torch": torch_init,
}

# Record modules that have been triggered
triggered = {}


class ProbingLoader(importlib.abc.Loader):
    """Custom loader that executes callbacks immediately after module loading"""

    def __init__(self, original_loader, fullname):
        self.original_loader = original_loader
        self.fullname = fullname

    def create_module(self, spec):
        # Delegate module creation to the original loader
        return (
            self.original_loader.create_module(spec)
            if hasattr(self.original_loader, "create_module")
            else None
        )

    def exec_module(self, module):
        # First let the original loader execute the module
        if hasattr(self.original_loader, "exec_module"):
            self.original_loader.exec_module(module)
        elif hasattr(self.original_loader, "load_module"):
            self.original_loader.load_module(self.fullname)

        # After module execution, immediately trigger the callback
        if self.fullname in register and self.fullname not in triggered:
            triggered[self.fullname] = True
            try:
                # register[self.fullname]()
                callbacks = register[self.fullname]
                if isinstance(callbacks, list):
                    for cb in callbacks:
                        cb()
                else:
                    callbacks()
            except Exception as e:
                print(f"Error in callback for {self.fullname}: {e}")


class ProbingFinder(importlib.abc.MetaPathFinder):
    """Custom finder for intercepting module imports and wrapping loaders"""

    def __init__(self):
        # Store original finders to restore the import chain
        self.original_meta_path = list(sys.meta_path)

    def find_spec(self, fullname, path, target=None):
        # If not a module we're interested in, skip it
        if fullname not in register:
            return None

        # Avoid recursive calls
        if fullname in sys._ProbingFinder_in_progress:  # type: ignore
            return None

        sys._ProbingFinder_in_progress.add(fullname)  # type: ignore
        try:
            # Temporarily remove self to avoid recursion
            sys.meta_path = [
                f for f in self.original_meta_path if not isinstance(f, ProbingFinder)
            ]

            # Use original finders to find the module
            spec = importlib.util.find_spec(fullname)

            # Restore meta_path
            sys.meta_path = list(self.original_meta_path)

            # If module is found, wrap its loader
            if spec is not None and spec.loader is not None:
                loader = ProbingLoader(spec.loader, fullname)
                spec.loader = loader

            return spec
        finally:
            sys._ProbingFinder_in_progress.remove(fullname)  # type: ignore
            # Always restore meta_path
            sys.meta_path = list(self.original_meta_path)


def register_module_callback(module_name, callback):
    """Register callback function for module import"""
    register[module_name] = callback

    # If the module is already imported, execute the callback immediately
    if module_name in sys.modules and module_name not in triggered:
        try:
            triggered[module_name] = True
            callback(sys.modules[module_name])
        except Exception as e:
            print(f"Error executing callback for {module_name}: {e}")


# Initialize recursion protection set
if not hasattr(sys, "_ProbingFinder_in_progress"):
    sys._ProbingFinder_in_progress = set()  # type: ignore


# Install import hook
def install():
    # Ensure it's only installed once
    for finder in sys.meta_path:
        if isinstance(finder, ProbingFinder):
            return finder

    # Create and install the hook
    finder = ProbingFinder()
    sys.meta_path.insert(0, finder)
    return finder


# Automatically install the hook
finder = install()
