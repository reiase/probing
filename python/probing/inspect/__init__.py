from .torch import get_torch_modules
from .torch import get_torch_tensors
from .torch import get_torch_optimizers

def get_dict():
    return {
        "int": 1,
        "float": 1.0,
        "str": "str",
    }
    
def get_list():
    return [
        1,
        1.0,
        "str",
    ]
    
def get_tuple():
    return (
        1,
        1.0,
        "str",
    )
    
def get_set():
    return {
        1,
        1.0,
        "str",
    }
    
def get_dict_list():
    return [
        {
            "int": 1,
            "float": 1.0,
            "str": "str",
        },
        {
            "int": 2,
            "float": 2.0,
            "str": "str2",
        },
    ]