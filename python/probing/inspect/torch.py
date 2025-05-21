import weakref
import time

tensor_cache = {}
module_cache = {}
optim_cache = {}

_last_full_refresh_time = 0
FULL_REFRESH_INTERVAL_SECONDS = 5 * 60


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
    global _last_full_refresh_time
    _last_full_refresh_time = time.time()

def _ensure_cache_updated():
    now = time.time()
    if now - _last_full_refresh_time > FULL_REFRESH_INTERVAL_SECONDS:
        refresh_cache()

def _build_active_list_and_clean_cache(cache_dict):
    """
    Builds a list of active items from the cache and cleans dead references.
    Returns the list of active items and a boolean indicating if dead refs were found.
    """
    active_items = []
    found_dead_ref = False
    # Iterate over a copy of items for safe deletion from the original cache_dict
    for k, v_ref in list(cache_dict.items()):
        obj = v_ref()  # Dereference the weakref
        if obj is not None:
            active_items.append({
                "id": k,
                "type": type(obj).__name__,
                "value": obj,
            })
        else:
            # Object has been garbage collected, remove from cache
            del cache_dict[k]
            found_dead_ref = True
    return active_items, found_dead_ref

def get_torch_modules():
    _ensure_cache_updated()  # Time-based refresh check

    active_items, found_dead_ref = _build_active_list_and_clean_cache(module_cache)

    if found_dead_ref:
        # print("Dead ref found in module_cache, triggering refresh_cache()") # For debugging
        refresh_cache()  # Force a full refresh
        # Rebuild the list from the now-refreshed cache
        active_items, _ = _build_active_list_and_clean_cache(module_cache)
    
    return active_items
    
def get_torch_tensors():
    _ensure_cache_updated()  # Time-based refresh check

    active_items, _ = _build_active_list_and_clean_cache(tensor_cache)
    return active_items

def get_torch_optimizers():
    _ensure_cache_updated()  # Time-based refresh check

    active_items, found_dead_ref = _build_active_list_and_clean_cache(optim_cache)

    if found_dead_ref:
        # print("Dead ref found in optim_cache, triggering refresh_cache()") # For debugging
        refresh_cache()  # Force a full refresh
        # Rebuild the list from the now-refreshed cache
        active_items, _ = _build_active_list_and_clean_cache(optim_cache)
        
    return active_items