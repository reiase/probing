import dataclasses
import functools
import re
from typing import Any, Optional, Type, Union

import probing

cache = {}


def camel_to_snake(name):
    """
    Convert CamelCase to snake_case.

    Examples
    --------
    >>> camel_to_snake("CamelCase")
    'camel_case'
    >>> camel_to_snake("SomeVeryLongClassName")
    'some_very_long_class_name'
    """
    s1 = re.sub("(.)([A-Z][a-z]+)", r"\1_\2", name)
    return re.sub("([a-z0-9])([A-Z])", r"\1_\2", s1).lower()


def table(name_or_class: Optional[Union[str, Type[Any]]] = None):
    """A decorator that converts a dataclass into a persistable table.

    This decorator adds database-like functionality to dataclasses. When applied to a dataclass,
    it creates or retrieves an ExternalTable with the dataclass name (or provided name) and adds
    methods for data persistence and retrieval operations.

    Parameters
    ----------
    name : Optional[str], default=None
        The name of the table to create or access. If None, the decorated class name will be used.
        When provided, the name will be converted to lowercase.

    Returns
    -------
    callable
        A decorator function that adds table functionality to the decorated dataclass.

    Methods Added
    ------------
    append(instance) : classmethod
        Adds a single instance to the table.
    append_many(instances) : classmethod
        Adds multiple instances to the table.
    take(n) : classmethod
        Retrieves n rows from the table.
    drop() : classmethod
        Deletes the table.
    save() : instancemethod
        Saves the current instance to the table.

    Raises
    ------
    TypeError
        If the decorated class is not a dataclass.
    ValueError
        If a table with the same name but different fields already exists.

    Examples
    --------
    >>> from dataclasses import dataclass
    >>> @table
    ... @dataclass
    ... class Point:
    ...     x: int
    ...     y: int
    >>> Point.append(Point(1, 2))
    >>> Point.take(10)[0][1]
    [1, 2]

    >>> Point(2, 3).save()
    >>> Point.take(10)[1][1]
    [2, 3]
    """

    if isinstance(name_or_class, str):
        cls = None
        name = name_or_class.lower()
    else:
        cls = name_or_class
        name = None

    def decorator(cls):
        if not dataclasses.is_dataclass(cls):
            raise TypeError(f"{cls} is not a dataclass")

        table_name = name or camel_to_snake(cls.__name__)
        fields = [f.name for f in dataclasses.fields(cls)]

        @functools.wraps(cls.__init__)
        def init_table():
            try:
                table = probing.ExternalTable.get(table_name)
                if table.names() != fields:
                    raise ValueError(
                        f"Table {table_name} already exists with different fields"
                    )
            except Exception:
                table = probing.ExternalTable(table_name, fields)
            cache[cls] = table
            return table

        @classmethod
        def append(cls, self):
            table = cache[cls]
            table.append(dataclasses.astuple(self))

        @classmethod
        def append_many(cls, self):
            table = cache[cls]
            table.append_many([dataclasses.astuple(i) for i in self])

        @classmethod
        def take(cls, n):
            table = cache[cls]
            return table.take(n)

        @classmethod
        def drop(cls):
            table = cache[cls]
            del cache[cls]
            table.drop(table_name)

        def save(self):
            cls.append(self)

        setattr(cls, "init_table", init_table)
        setattr(cls, "append", append)
        setattr(cls, "append_many", append_many)
        setattr(cls, "take", take)
        setattr(cls, "drop", drop)
        setattr(cls, "save", save)
        init_table()

        return cls

    if cls is not None:
        return decorator(cls)
    return decorator
