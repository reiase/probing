from .trace import probe

_ALL_ = [
    "init",
    "probe",
    "DebugConsole",
]

VERSION = "0.2.0"


def initialize_probing():
    """
    Initialize probing by loading the libprobing.so dynamic library.

    Raises:
        ImportError: If the library cannot be found or loaded.
    """
    import ctypes
    import pathlib
    import sys

    # Search paths for the library
    current_file = pathlib.Path(__file__).resolve()

    paths = [
        pathlib.Path(sys.executable).parent / "libprobing.so",
        current_file.parent / ".." / ".." / ".." / ".." / "bin" / "libprobing.so",
        pathlib.Path.cwd() / "libprobing.so",
        pathlib.Path.cwd() / "target" / "debug" / "libprobing.so",
        pathlib.Path.cwd() / "target" / "release" / "libprobing.so",
    ]

    # Try loading the library from each path while preserving the actual
    # dynamic-loader error for diagnostics.
    load_errors = []
    for path in paths:
        if path.exists():
            try:
                return ctypes.CDLL(str(path))
            except OSError as error:
                load_errors.append(f"{path}: {error}")

    searched_paths = ", ".join(str(path) for path in paths)
    error_details = "; ".join(load_errors) or "no candidate library exists"
    raise ImportError(
        f"Could not load libprobing.so. Searched in: {searched_paths}. "
        f"Loader errors: {error_details}"
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
