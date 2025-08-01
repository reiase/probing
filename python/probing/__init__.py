# from .repl import DebugConsole
# from .trace import probe

# _ALL_ = [
#     "init",
#     "probe",
#     # "DebugConsole",
# ]

VERSION = "0.2.0"


def initialize_probing():
    """
    Initialize probing by loading the libprobing.so dynamic library.

    This function searches for the library in two primary locations:
    1. Next to the package (`probing/libprobing.so`), which is the standard location
       when the wheel is installed.
    2. In the cargo target directory (`target/[debug|release]/libprobing.so`),
       which is used for local development.

    Raises:
        ImportError: If the library cannot be found or loaded.
    """
    import ctypes
    import os
    import pathlib

    # Path when installed from wheel
    current_dir = pathlib.Path(__file__).parent
    library_path = current_dir / "libprobing.so"

    search_paths = [library_path]

    if not library_path.exists():
        # If not found, try development path
        try:
            # Determine build profile
            target_dir = "debug" if "DEBUG" in os.environ else "release"

            # Find project root by looking for 'Cargo.toml'
            project_root = current_dir
            while not (project_root / "Cargo.toml").exists():
                if project_root == project_root.parent:
                    # Reached filesystem root, stop
                    project_root = None
                    break
                project_root = project_root.parent

            if project_root:
                dev_path = project_root / "target" / target_dir / "libprobing.so"
                search_paths.append(dev_path)
                if dev_path.exists():
                    library_path = dev_path
        except Exception:
            # Ignore errors during dev path resolution
            pass

    if not library_path.exists():
        raise ImportError(
            f"Could not find libprobing.so. Searched in: {', '.join(map(str, search_paths))}"
        )

    try:
        # On Linux, we need to load the library with RTLD_GLOBAL to make sure
        # that the symbols are available for other libraries.
        # This is especially important for extensions.
        if hasattr(ctypes, "RTLT_GLOBAL"):
             ctypes.CDLL(str(library_path), mode=ctypes.RTLD_GLOBAL)
        else:
             ctypes.CDLL(str(library_path)) # Fallback for other OS
    except Exception as e:
        raise ImportError(f"Could not load libprobing.so from {library_path}: {e}")


initialize_probing()

import probing.hooks.import_hook
import probing.inspect

from probing.core.engine import query
from probing.core.engine import load_extension

__all__ = [
    "query",
    "load_extension",
    "VERSION",
]
