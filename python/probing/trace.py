import ctypes
import functools
import os
import sys
import threading
import json
from types import FrameType
from types import FunctionType
from typing import Any, AnyStr
from types import ModuleType

thread_global = threading.local()
internal_directories = os.path.dirname((lambda: 0).__code__.co_filename)

traced_functions = {}


def probe(func=None, watch=[], depth=1):
    if func is not None:

        @functools.wraps(func)
        def wrapper(*args, **kwargs):
            tracer = ProbingTracer(depth, watch)
            with tracer:
                return func(*args, **kwargs)

        return wrapper
    else:

        def decorator(func):
            @functools.wraps(func)
            def wrapper(*args, **kwargs):
                tracer = ProbingTracer(depth, watch)
                with tracer:
                    return func(*args, **kwargs)

            return wrapper

        return decorator


class ProbingTracer:
    def __init__(self, depth=1, watch=[]):
        self.depth = depth
        self.count_calls = 0
        self.count_returns = 0
        self.watch = watch
        self.watch_impl = {}

    def on_call(self):
        self.count_calls += 1

    def on_return(self):
        self.count_returns += 1

    def _outof_depth(self):
        depth = self.count_calls - self.count_returns
        return depth > self.depth

    def _is_internal_frame(self, frame):
        return frame.f_code.co_filename.startswith(internal_directories)

    def __enter__(self):
        tracer_stack = thread_global.__dict__.setdefault("tracer_stack", [])
        tracer_stack.append(sys.gettrace())
        sys.settrace(self.trace)

    def __exit__(self, exc_type, exc_val, exc_tb):
        tracer_stack = thread_global.tracer_stack
        sys.settrace(tracer_stack.pop())

    def trace(self, frame: FrameType, event: AnyStr, arg: Any):
        import torch

        # print(
        #     f"Event: {event}, Frame: {frame}, Arg: {arg}, name: {frame.f_code.co_name}"
        # )

        if event == "call":
            self.on_call()
            if self._outof_depth():
                frame.f_locals["__trace_checkpoint__"] = TracerCheckpoint(
                    self.on_return
                )
                return None
            if not self.watch_impl and self.watch:
                self.watch_impl = {
                    k: id(frame.f_locals.get(k, None)) for k in self.watch
                }
                print(
                    f"In {frame.f_code.co_name} from {frame.f_code.co_filename} line {frame.f_lineno}:"
                )
                print(f"  start watching variables: {[k for k in self.watch_impl]}")
            return self.trace
        if event == "return":
            self.on_return()
            for k in self.watch:
                if k in frame.f_locals and isinstance(
                    frame.f_locals[k], FakeProbingTensor
                ):
                    frame.f_locals[k] = torch.Tensor(frame.f_locals[k])
                    ctypes.pythonapi.PyFrame_LocalsToFast(
                        ctypes.py_object(frame), ctypes.c_int(0)
                    )
            return self.trace
        if self._is_internal_frame(frame):
            return None

        for k in self.watch:
            if (
                k in frame.f_locals
                and isinstance(frame.f_locals[k], torch.Tensor)
                and (not isinstance(frame.f_locals[k], FakeProbingTensor))
            ):
                frame.f_locals[k] = ProbingTensor(frame.f_locals[k])
                ctypes.pythonapi.PyFrame_LocalsToFast(
                    ctypes.py_object(frame), ctypes.c_int(0)
                )
        for k, v in self.watch_impl.items():
            if k in frame.f_locals and id(frame.f_locals[k]) != v:
                print(f"probing: variable update {k} = {frame.f_locals[k]}")
                self.watch_impl[k] = id(frame.f_locals[k])
        return self.trace


class TracerCheckpoint:
    def __init__(self, callback=None):
        self.trace = sys.gettrace()
        self.callback = callback
        sys.settrace(None)

    def __del__(self):
        if self.callback:
            self.callback()
        sys.settrace(self.trace)


class FakeProbingTensor:
    pass


__ProbingTensor = None


def ProbingTensor(*args, **kwargs):
    import torch

    class _ProbingTensor(torch.Tensor, FakeProbingTensor):
        def __format__(self, format_spec):
            return f"{self.item().__format__(format_spec)}"

        @classmethod
        def __torch_function__(cls, func, types, args=(), kwargs=None):
            if kwargs is None:
                kwargs = {}
            if (
                func is not torch.Tensor.__repr__
                # and func is not torch.Tensor.__format__
                and func is not torch.Tensor.__str__
                and func.__name__.endswith("_")
                and not func.__name__.startswith("__")
            ):
                old_val = f"{args}"
                ret = super().__torch_function__(func, types, args, kwargs)
                ret_val = f"{args}"
                print(
                    f"probing: tensor update with {func.__name__}: {old_val} => {ret_val}"
                )
                return ret
            return super().__torch_function__(func, types, args, kwargs)

    global __ProbingTensor
    if __ProbingTensor is None:
        __ProbingTensor = _ProbingTensor
    return __ProbingTensor(*args, **kwargs)


def list_traceable(prefix=None):
    if prefix is None:
        filter = lambda x: True
    else:
        filter = lambda x: x.startswith(prefix)

    whitelist = [
        "__main__",
    ]
    blacklist = [
        "numpy",
        "typing",
        "typing.io",
        "typing_extensions",
    ]
    traceable_functions = []
    travel_history = set()

    def getname(obj):
        if hasattr(obj, "__name__") and isinstance(obj.__name__, str):
            return obj.__name__
        else:
            return None

    def travel(obj, prefix=""):
        if id(obj) in travel_history or prefix in blacklist:
            return
        if prefix.startswith("torch"):
            if not (
                prefix.startswith("torch.nn")
                or prefix.startswith("torch.cuda")
                or prefix.startswith("torch.distributed")
                or prefix.startswith("torch.optim")
            ):
                return
        travel_history.add(id(obj))
        if hasattr(obj, "__dict__"):
            for k, v in obj.__dict__.items():
                name = getname(v)
                if name is not None and not name.startswith("__") and filter(name):
                    if isinstance(v, FunctionType) and hasattr(v, "__code__"):
                        traceable_functions.append(f"{prefix}.{k}")
                    else:
                        if not isinstance(v, ModuleType):
                            travel(v, f"{prefix}.{k}")

    for k, v in sys.modules.items():
        if isinstance(v, ModuleType) and hasattr(v, "__spec__"):
            if k in whitelist:
                travel(v, k)
                continue
            if v.__spec__ is None or not "site-packages" in v.__spec__.origin:
                continue
            if isinstance(k, str) and not k.startswith("__"):
                travel(v, k)
    return json.dumps(traceable_functions, indent=2)


def trace(func_or_name, watch=[], depth=1, callback=None):
    def get_func(name):
        names = name.split(".")
        parent = sys.modules.get(names[0], None)
        names = names[1:]
        while parent is not None and len(names) > 0:
            if hasattr(parent, names[0]):
                if len(names) == 1:
                    return parent, getattr(parent, names[0]), names[0]
                parent = getattr(parent, names[0])
                names = names[1:]
            else:
                raise ValueError(f"{names[0]} not found in {parent}.")

    if isinstance(func_or_name, str):
        if func_or_name in traced_functions:
            print(f"Function {func_or_name} is already being traced.")
            return
        try:
            parent, func, name = get_func(func_or_name)
            traced_functions[func_or_name] = func
            func = probe(func, watch=watch, depth=depth)
            parent.__setattr__(name, func)
        except Exception:
            print(f"Function {func_or_name} not found.")
            return
    else:
        raise NotImplementedError("Only string names are supported for tracing.")


def untrace(func_or_name):
    def get_func(name):
        names = name.split(".")
        parent = sys.modules.get(names[0], None)
        names = names[1:]
        while parent is not None and len(names) > 0:
            if hasattr(parent, names[0]):
                if len(names) == 1:
                    return parent, getattr(parent, names[0]), names[0]
                parent = getattr(parent, names[0])
                names = names[1:]
            else:
                raise ValueError(f"{names[0]} not found in {parent}.")

    if isinstance(func_or_name, str):
        if func_or_name not in traced_functions:
            print(f"Function {func_or_name} is not being traced.")
            return
        try:
            parent, func, name = get_func(func_or_name)
            func = traced_functions.pop(func_or_name)
            parent.__setattr__(name, func)
        except Exception:
            print(f"Function {func_or_name} not found.")
            return
    else:
        raise NotImplementedError("Only string names are supported for tracing.")


def show_trace():
    return json.dumps([x for x in traced_functions.keys()], indent=2)
