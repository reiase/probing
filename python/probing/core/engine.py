"""
Probing Core Engine Module.

This module provides the core functionality for executing SQL queries and
loading Rust extensions in the Probing library. It serves as the primary
interface between Python code and the underlying Rust implementation.

The module offers functions to:
- Execute SQL queries against data sources
- Load and initialize Rust extensions for extended functionality

Examples:
    >>> import probing
    >>> # Execute a simple SQL query
    >>> df = probing.query("SHOW TABLES")
    >>> type(df)
    <class 'pandas.core.frame.DataFrame'>

    >>> # Load a custom extension
    >>> mod = probing.load_extension("probing.ext.example")
    >>> type(mod)
    <class 'module'>
"""

import importlib
import json
import sys
import traceback


def query(sql: str) -> "DataFrame":  # type: ignore
    """
    Execute a SQL query and return the result as a pandas DataFrame.

    This function sends the SQL query to the underlying Rust implementation and
    processes the returned JSON data into a pandas DataFrame for easy manipulation
    in Python. If the result cannot be converted to a DataFrame, the raw JSON
    result is returned instead.

    Args:
        sql (str): The SQL query string to execute.

    Returns:
        pandas.DataFrame: The query results as a DataFrame. If conversion fails,
                         the raw JSON string is returned instead.

    Raises:
        RuntimeError: If the query execution fails in the Rust layer.
        ValueError: If the SQL statement is invalid.

    Examples:
        >>> import probing
        >>> df = probing.query("SELECT 1 AS a, 2 AS b")
        >>> print(df)
           a  b
        0  1  2
    """
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

    This function dynamically imports and initializes Rust extensions that enhance
    the functionality of the probing library. Extensions are specified using Python
    import path notation and must be compatible with the probing library's extension API.

    Args:
        statement (str): The Python import path to the extension, typically ending with an
                         initialization function (e.g., "myextension.module.init").

    Returns:
        Any: The return value of the called extension function/statement.

    Raises:
        ImportError: If the extension module cannot be imported.
        AttributeError: If the specified attribute doesn't exist in the module.

    Examples:
        >>> import probing
        >>> mod = probing.load_extension("probing.ext.example")
        >>> type(mod)
        <class 'module'>
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
