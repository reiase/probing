from dataclasses import dataclass

from probing.table import table

@table
@dataclass
class ExampleExt:
    id: int
    name: str
    
def init():
    print("Initializing ExampleExt")
    pass

def deinit():
    print("Deinitializing ExampleExt")
    ExampleExt.drop()
