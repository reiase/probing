import queue
import random
import threading
import time
from dataclasses import dataclass, field

from typing import Any, List

import torch
import torch.mps

try:
    import probing

    TORCH_PROFILING = probing.ExternalTable(
        "torch_profiling", ["module_name", "stage", "duration"]
    )

except:
    TORCH_PROFILING = None


def module_cache(func):
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


@module_cache
def _get_module_fullname(m):
    return f"{m.__module__}.{m.__class__.__name__}"


@module_cache
def _get_module_params(m):
    return {k: TensorDef.create(v) for k, v in m.named_parameters(recurse=False)}


@module_cache
def _is_container(m):
    return isinstance(m, torch.nn.Module) and len(list(m.children())) > 0


def module_name(m, name=None):
    global NAME_CACHE
    mid = id(m)
    if mid in NAME_CACHE:
        return NAME_CACHE[mid]
    elif name is not None:
        NAME_CACHE[mid] = name
        return name
    return "unknown_module"


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


NAME_CACHE = {}
HOOK_CACHE = {}
EVENT_COUNT = 0
SKIP_COUNT = 0


class SpanPoller:
    def __init__(self) -> None:
        self.queue = queue.Queue(maxsize=1024)
        self.stop = False
        self.thread = threading.Thread(target=self.run, daemon=True)
        self.thread.start()

    @try_catch(3)
    def add(self, span):
        self.queue.put_nowait(span)

    def run(self):
        span = None
        while not self.stop:
            if span is None:
                span = self.queue.get()
            if span.poll():
                if TORCH_PROFILING is not None:
                    TORCH_PROFILING.append([span.module, span.kind, span.duration])
                # print(f"=={EVENT_COUNT}== {span}")
                span = None
            else:
                time.sleep(0.1)


@dataclass
class TensorDef:
    shape: tuple = ()
    dtype: Any = None

    @staticmethod
    def create(t):
        return TensorDef(t.shape, t.dtype)


@dataclass(init=False)
class Span:
    id: int = field(repr=False, default=None)
    module: torch.nn.Module = field(repr=True, default=None)
    kind: str = field(repr=True, default=None)

    start_event: Any = field(repr=False, default=None)
    end_event: Any = field(repr=False, default=None)

    is_container: bool = field(repr=False, default=False)

    inputs: List[TensorDef] = field(default_factory=list)
    output: TensorDef = None
    params: dict = field(default_factory=dict)

    duration: float = field(repr=True, default=None)

    def __init__(
        self, m, kind="forward", inputs=None, output=None, params=None
    ) -> None:
        self.id = id(m)
        self.module = module_name(m)
        self.kind = kind
        self.inputs = inputs
        self.output = output
        self.params = params
        self.is_container = _is_container(m)

    @staticmethod
    def forward(m, inputs, output=None, params=None):
        return Span(m, "forward", inputs, output, params)

    @staticmethod
    def backward(m, inputs, output=None, params=None):
        return Span(m, "backward", inputs, output, params)

    def begin(self, dm):
        self.start_event = dm.Event(enable_timing=True)
        self.start_event.record()
        return self

    def end(self, dm):
        if self.start_event is not None:
            self.end_event = dm.Event(enable_timing=True)
            self.end_event.record()
            return self
        return None

    def poll(self):
        if self.start_event is None:
            return True
        if not self.start_event.query():
            return False
        if self.end_event is None or not self.end_event.query():
            return False
        self.duration = self.start_event.elapsed_time(self.end_event)
        return True


class Tracer:
    POLLER = SpanPoller()

    def __init__(self, sample_ratio=1.0) -> None:
        self.spans = []
        self.device_manager = torch.cuda if torch.cuda.is_available() else torch.mps
        self.sample_ratio = sample_ratio

    def begin_span(self, span):
        if self.sample_ratio > 0 and (random.uniform(0, 1) < self.sample_ratio):
            span.begin(self.device_manager)
        self.spans.append(span)
        return span

    def end_span(self, m):
        global EVENT_COUNT
        global SKIP_COUNT
        span = self.spans.pop().end(self.device_manager)
        if span is not None:
            EVENT_COUNT += 1
            Tracer.POLLER.add(span)
        else:
            SKIP_COUNT += 1

    @staticmethod
    @try_catch(3)
    def pre_forward_hook(m, i):
        params = _get_module_params(m)
        inputs = [TensorDef.create(t) if isinstance(t, torch.Tensor) else t for t in i]
        TRACER.begin_span(Span.forward(m, inputs, params=params))

    @staticmethod
    @try_catch(3)
    def pre_backward_hook(m, i):
        params = _get_module_params(m)
        inputs = [TensorDef.create(t) if isinstance(t, torch.Tensor) else t for t in i]
        TRACER.begin_span(Span.backward(m, inputs, params=params))

    @staticmethod
    @try_catch(3)
    def post_hook(m, i, o):
        TRACER.end_span(m)


TRACER = Tracer()


def module_analysis(m, prefix=""):
    if not isinstance(m, torch.nn.Module):
        return
    for n, s in m.named_children():
        name = f"{prefix}.{n}" if prefix != "" else n
        module_name(s, name)
        module_analysis(s, name)


def torch_profiling(sample_ratio=1.0):
    TRACER.sample_ratio = sample_ratio

    def try_install():
        import gc

        objs = [obj for obj in gc.get_objects() if isinstance(obj, torch.nn.Module)]

        children = set()

        def walk(obj):
            if hasattr(obj, "children"):
                for child in obj.children():
                    children.add(id(child))
                    walk(child)

        for obj in objs:
            walk(obj)
        toplevel = [obj for obj in objs if id(obj) not in children]
        for m in toplevel:
            if id(m) not in HOOK_CACHE:
                install_hooks(m)

    def worker():
        sleep = 1
        while True:
            try_install()
            time.sleep(sleep)
            sleep *= 2

    thread = threading.Thread(target=worker, daemon=True)
    thread.start()


def install_hooks(m: torch.nn.Module = None):
    if m is None:
        torch.nn.modules.module.register_module_forward_pre_hook(
            Tracer.pre_forward_hook
        )
        torch.nn.modules.module.register_module_forward_hook(Tracer.post_hook)
        # TODO: global bw hooks is not supported
        # torch.nn.modules.module.register_module_full_backward_pre_hook(Tracer.pre_backward_hook)
        # torch.nn.modules.module.register_module_full_backward_hook(Tracer.post_backward_hook)
    else:
        if id(m) in HOOK_CACHE:
            return
        module_analysis(m)
        h1 = m.register_forward_pre_hook(Tracer.pre_forward_hook)
        h2 = m.register_forward_hook(Tracer.post_hook)
        module_name = _get_module_fullname(m)
        if not module_name.endswith("FusedScaleMaskSoftmax"):
            h3 = m.register_full_backward_pre_hook(Tracer.pre_backward_hook)
            h4 = m.register_full_backward_hook(Tracer.post_hook)
            HOOK_CACHE[id(m)] = (h1, h2, h3, h4)
        else:
            HOOK_CACHE[id(m)] = (h1, h2)
        for s in m.children():
            install_hooks(s)
