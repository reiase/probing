from dataclasses import dataclass

from probing.core import table


@table
@dataclass
class ExampleExt:
    id: int
    name: str


def init():
    print("Initializing ExampleExt")
    ExampleExt.init_table()
    pass


def deinit():
    print("Deinitializing ExampleExt")
    ExampleExt.drop()
