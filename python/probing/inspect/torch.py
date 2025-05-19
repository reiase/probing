import weakref

tensor_cache = {}
module_cache = {}
optim_cache = {}


def update_cache(x):
    import torch

    idx = id(x)
    if isinstance(x, torch.Tensor):
        if idx not in tensor_cache:
            tensor_cache[idx] = weakref.ref(x)
        return tensor_cache[idx]
    if isinstance(x, torch.nn.Module):
        if idx not in module_cache:
            module_cache[idx] = weakref.ref(x)
        return module_cache[idx]
    if isinstance(x, torch.optim.Optimizer):
        if idx not in optim_cache:
            optim_cache[idx] = weakref.ref(x)
        return optim_cache[idx]


def refresh_cache():
    import gc

    for obj in gc.get_objects():
        update_cache(obj)

def get_torch_modules():
    refresh_cache()
    return [
        {
            "id": k,
            "type": type(v()).__name__,
            "value": v(),
        }
        for k,v in module_cache.items()
    ]
    
def get_torch_tensors():
    refresh_cache()
    return [
        {
            "id": k,
            "type": type(v()).__name__,
            "value": v(),
        }
        for k,v in tensor_cache.items()
    ]

def get_torch_optimizers():
    refresh_cache()
    return [
        {
            "id": k,
            "type": type(v()).__name__,
            "value": v(),
        }
        for k,v in optim_cache.items()
    ]