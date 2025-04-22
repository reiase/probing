import torch

from .types import TensorDef

NAME_CACHE = {}


def module_name(m, name=None):
    global NAME_CACHE
    mid = id(m)
    if mid in NAME_CACHE:
        return NAME_CACHE[mid]
    elif name is not None:
        NAME_CACHE[mid] = name
        return name
    return None


def try_catch(maxtry=3):
    def decorator(func):
        _maxtry = maxtry

        def wrapper(*args, **kwargs):
            try:
                return func(*args, **kwargs)
            except:
                nonlocal _maxtry
                _maxtry -= 1
                if _maxtry > 0:
                    import traceback

                    traceback.print_exc()

        return wrapper

    return decorator


def module_analysis(m, prefix=""):
    if not isinstance(m, torch.nn.Module):
        return
    for n, s in m.named_children():
        name = f"{prefix}.{n}" if prefix != "" else n
        module_name(s, name)
        module_analysis(s, name)


def _cache(func):
    cache = {}

    def wrapper(m, value=None):
        nonlocal cache
        mid = id(m)
        if value is not None:
            cache[mid] = value
        elif mid not in cache:
            cache[mid] = func(m)
            return cache[mid]
        else:
            return cache[mid]

    return wrapper


@_cache
def module_get_fullname(m):
    return f"{m.__module__}.{m.__class__.__name__}"


@_cache
def module_get_params(m):
    return {k: TensorDef.create(v) for k, v in m.named_parameters(recurse=False)}


@_cache
def module_is_container(m):
    return isinstance(m, torch.nn.Module) and len(list(m.children())) > 0
