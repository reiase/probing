import importlib
import json
import sys
import traceback


def query(sql: str) -> "DataFrame":  # type: ignore
    import sys

    probing = sys.modules["probing"]

    ret = probing.query_json(sql)
    try:
        import json

        import pandas as pd

        data = json.loads(ret)

        data = {k: list(v.values())[0] for k, v in zip(data["names"], data["cols"])}
        return pd.DataFrame(data)
    except:
        import traceback

        traceback.print_exc()
        return ret


def load_extension(statement: str):
    """
    Load a Rust extension into the probing library.

    Args:
        statement (str): The SQL statement to load the extension.

    Returns:
        None
    """

    import importlib
    import sys

    parts = statement.split(".")
    if parts[0] not in sys.modules:
        importlib.import_module(parts[0])
    root = sys.modules[parts[0]]
    module = f"{parts[0]}"
    for part in parts[1:]:
        if not hasattr(root, part):
            importlib.import_module(module + "." + part)
        module = f"{module}.{part}"

    return eval(
        statement,
        None,
        {
            parts[0]: sys.modules[parts[0]],
        },
    )
