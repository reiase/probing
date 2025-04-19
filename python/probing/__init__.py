from .trace import probe
from .repl import DebugConsole

_ALL_ = [
    "init",
    "probe",
    "DebugConsole",
]

VERSION = "0.2.0"

def init():
    """
    Initialize probing by loading the libprobing.so dynamic library.
    
    Raises:
        ImportError: If the library cannot be found or loaded.
    """
    import ctypes
    import pathlib
    import sys
    
    # Search paths for the library
    paths = [
        pathlib.Path(sys.executable).parent / "libprobing.so",
        pathlib.Path.cwd() / "libprobing.so",
        pathlib.Path.cwd() / "target" / "debug" / "libprobing.so",
        pathlib.Path.cwd() / "target" / "release" / "libprobing.so",
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
        f"Could not find or load libprobing.so. Searched in: {', '.join(str(p) for p in paths)}"
    )
    
init()

def query(sql: str) -> "DataFrame":
    import sys
    probing = sys.modules["probing"]
    
    ret = probing.query_json(sql)
    try:
        import pandas as pd
        import json
        data = json.loads(ret)
        
        data = {k: list(v.values())[0] for k, v in zip(data["names"], data["cols"])}
        return pd.DataFrame(data)
    except:
        import traceback
        traceback.print_exc()
        return ret