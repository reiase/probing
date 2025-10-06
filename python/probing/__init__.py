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
    Initialize probing by loading the dynamic library.

    Raises:
        ImportError: If the library cannot be found or loaded.
    """
    import ctypes
    import pathlib
    import sys

    # Determine library name based on OS
    if sys.platform == "darwin":
        lib_name = "libprobing.dylib"
    else:
        lib_name = "libprobing.so"

    # Search paths for the library
    current_file = pathlib.Path(__file__).resolve()

    paths = [
        pathlib.Path(sys.executable).parent / lib_name,
        current_file.parent / lib_name,
        pathlib.Path.cwd() / lib_name,
        pathlib.Path.cwd() / "target" / "debug" / lib_name,
        pathlib.Path.cwd() / "target" / "release" / lib_name,
    ]

    # Try loading the library from each path
    for path in paths:
        if path.exists():
            try:
                return ctypes.CDLL(str(path))
            except Exception:
                continue  # Try the next path if loading fails

    # If we get here, the library wasn't found or couldn't be loaded
    raise ImportError(
        f"Could not find or load {lib_name}. Searched in: {', '.join(str(p) for p in paths)}"
    )


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
