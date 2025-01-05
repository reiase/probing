import logging
import queue
import random
import threading
import time

import torch
import torch.mps
from hyperparameter import auto_param

from xinsight.types.events import (
    BackwardEndEvent,
    BackwardStartEvent,
    Event,
    ForwardEndEvent,
    ForwardStartEvent,
    TenserDef,
)


def _get_fullname(m):
    return f"{m.__module__}.{m.__class__.__name__}"


def _is_container(m):
    return isinstance(m, torch.nn.Module) and len(list(m.children())) > 0


class SpanPoller:
    def __init__(self) -> None:
        self.queue = queue.Queue()
        self.stop = False

    def add(self, span):
        self.queue.put(span)

    def start(self):
        self.thread = threading.Thread(target=self.run, daemon=True)
        self.thread.start()
        return self

    def run(self):
        span = None
        log = None
        while not self.stop:
            if span is None:
                try:
                    log = self.queue.get(timeout=10)
                except:
                    time.sleep(0.001)
                    continue
                if isinstance(log, SpanEnd):
                    span = log
                else:
                    Tracer.get().logger.info(log)
                    continue
            if span.poll():
                span.elapsed_time()
                Tracer.get().logger.info(span)
                span = None
            else:
                time.sleep(0.001)


class Span:
    POLLER = SpanPoller().start()

    def __init__(self, m, enable_timing=True) -> None:
        self.name = _get_fullname(m)
        self.id = id(m)
        self.is_container = _is_container(m)
        self.start_event = None
        self.end_event = None
        self.enable_timing = enable_timing
        self.duration = None

    def __repr__(self) -> str:
        if self.duration is None:
            return f'SpanStartEvent("{self.name}", {self.id})'
        raise NotImplementedError
        return f"SpanEnd({self.name}, {self.id}, {self.duration})"

    def begin(self, dm):
        if self.enable_timing and self.is_container:  # not self.name.startswith("torch.nn."):
            self.start_event = dm.Event(enable_timing=True)
            self.start_event.record()
            return self
        return None

    def end(self, dm):
        if self.enable_timing and self.is_container:  # not self.name.startswith("torch.nn."):
            self.end_event = dm.Event(enable_timing=True)
            self.end_event.record()
            return SpanEnd(self)
        return None

    def poll(self):
        if not self.enable_timing:
            return True
        if self.start_event is None or not self.start_event.query():
            return False
        if self.end_event is None or not self.end_event.query():
            return False
        return True

    def elapsed_time(self):
        if not self.enable_timing:
            self.duration = 0
        try:
            self.duration = self.start_event.elapsed_time(self.end_event)
        except:
            self.duration = 0

    def finish(self):
        event = Event()
        event.name = "SpanEndEvent"
        event.module = self.name
        event.inputs = [(self.id, self.duration)]
        return event

    def random_enable(self, threshold=1.0):
        self.enable_timing = random.random() < float(threshold)
        return self


class SpanEnd:
    def __init__(self, span) -> None:
        self.span = span

    def poll(self):
        return self.span.poll()

    def elapsed_time(self):
        return self.span.elapsed_time()

    def __repr__(self) -> str:
        return f'SpanEndEvent("{self.span.name}", {self.span.id}, {self.span.duration})'


class Tracer:
    tls = threading.local()

    @auto_param("xinsight.tracer")
    def __init__(self, output="", enable_timing=True, sample_ratio=1.0) -> None:
        if not hasattr(Tracer.tls, "trace"):
            Tracer.tls.trace = self

        self.spans = []
        self.device_manager = torch.cuda if torch.cuda.is_available() else torch.mps
        self.logger = logging.getLogger("tracer")
        if output != "":
            self.logger.setLevel(logging.INFO)
            self.logger.addHandler(logging.FileHandler(output, mode="w"))
        self.enable_timing = enable_timing
        self.sample_ratio = sample_ratio

    @staticmethod
    def get():
        if not hasattr(Tracer.tls, "trace"):
            Tracer()
        return Tracer.tls.trace

    @staticmethod
    def init(output="", enable_timing=True, sample_ratio=1.0):
        Tracer(output=output, enable_timing=enable_timing, sample_ratio=sample_ratio)

    def log(self, event):
        if event is not None:
            Span.POLLER.add(event)

    def begin_forward(self, m: torch.nn.Module, inputs):
        self.begin_span(m)
        params = {k: TenserDef(tuple(v.shape), v.dtype) for k, v in m.named_parameters(recurse=False)}
        self.log(ForwardStartEvent(m, inputs, params=params))

    def end_forward(self, m, inputs, outputs):
        params = {k: TenserDef(tuple(v.shape), v.dtype) for k, v in m.named_parameters(recurse=False)}
        self.log(ForwardEndEvent(m, outputs, params=params))
        self.end_span(m)

    def begin_backward(self, m, inputs):
        self.begin_span(m)
        params = {k: TenserDef(tuple(v.shape), v.dtype) for k, v in m.named_parameters(recurse=False)}
        self.log(BackwardStartEvent(m, inputs, params=params))

    def end_backward(self, m, inputs, outputs):
        params = {k: TenserDef(tuple(v.shape), v.dtype) for k, v in m.named_parameters(recurse=False)}
        self.log(BackwardEndEvent(m, outputs, params=params))
        self.end_span(m)

    def begin_span(self, m):
        if self.enable_timing:
            span = Span(m).random_enable(self.sample_ratio)
        else:
            span = Span(m, enable_timing=False)
        self.spans.append(span)
        self.log(span.begin(self.device_manager))

    def end_span(self, m):
        span = self.spans.pop()
        self.log(span.end(self.device_manager))

    @staticmethod
    def pre_forward_hook(m, i):
        Tracer.get().begin_forward(m, i)

    @staticmethod
    def post_forward_hook(m, i, o):
        Tracer.get().end_forward(m, i, o)

    @staticmethod
    def pre_backward_hook(m, i):
        Tracer.get().begin_backward(m, i)

    @staticmethod
    def post_backward_hook(m, i, o):
        Tracer.get().end_backward(m, i, o)


def install_hooks(m: torch.nn.Module = None):
    if m is None:
        torch.nn.modules.module.register_module_forward_pre_hook(Tracer.pre_forward_hook)
        torch.nn.modules.module.register_module_forward_hook(Tracer.post_forward_hook)
        # TODO: global bw hooks is not supported
        # torch.nn.modules.module.register_module_full_backward_pre_hook(Tracer.pre_backward_hook)
        # torch.nn.modules.module.register_module_full_backward_hook(Tracer.post_backward_hook)
    else:
        m.register_forward_pre_hook(Tracer.pre_forward_hook)
        m.register_forward_hook(Tracer.post_forward_hook)
        if not _get_fullname(m).startswith("torch.nn"):
            m.register_full_backward_pre_hook(Tracer.pre_backward_hook)
            m.register_full_backward_hook(Tracer.post_backward_hook)
        for s in m.children():
            install_hooks(s)
