import random
import time
from dataclasses import dataclass

from probing.table import table

from .module_utils import module_name
from .types import BaseTracer


@table("torch_trace")
@dataclass
class TorchTrace:
    step: int = None
    seq: int = None
    module: str = None
    stage: str = None
    allocated: float = 0.0
    max_allocated: float = 0.0
    cached: float = 0.0
    max_cached: float = 0.0
    time_offset: float = 0.0
    duration: float = 0.0


class DelayedRecord:
    def __init__(self, record, events):
        self.record = record
        self.events = events

    def save(self):
        try:
            if self.events is not None:
                start, end = self.events
                self.record.duration = end.elapsed_time(start) / 1000.0
            self.record.save()
        except Exception as e:
            print(f"Error saving trace: {e}")


def mem_stats() -> TorchTrace:
    import torch

    MB = 1024 * 1024
    return TorchTrace(
        allocated=torch.cuda.memory_allocated() / MB,
        cached=torch.cuda.memory_reserved() / MB,
        max_allocated=torch.cuda.max_memory_allocated() / MB,
        max_cached=torch.cuda.max_memory_reserved() / MB,
    )


class Timer:
    def __init__(self, sync: bool = False, **kwargs):
        import torch

        self.has_cuda = torch.cuda.is_available()
        self.sync = sync
        self.events = {}  # GPU timers
        self.step_start = None

        super().__init__(**kwargs)

    def begin_timing(self, mod, stage) -> float:
        # Synchronize if needed for more accurate timing
        if self.sync and self.has_cuda:
            _cuda_sync()

        if self.offset() == 0:
            self.step_start = time.time()
            time_offset = 0.0
        else:
            time_offset = time.time() - self.step_start

        if self.has_cuda:
            self.events[(id(mod), stage)] = _cuda_event()
        return time_offset

    def end_timing(self, mod, stage) -> tuple:
        # Synchronize if needed for more accurate timing
        if self.sync and self.has_cuda:
            _cuda_sync()

        time_offset = time.time() - self.step_start
        key = (id(mod), stage)

        if key in self.events:
            return time_offset, (self.events.pop(key), _cuda_event())
        return time_offset, None


class Sampler:
    def __init__(self, mode="ordered", rate=0.05, **kwargs):
        # Strategy configuration
        self.mode = mode
        self.rate = rate

        # Module tracking state
        self.mod_names = {}  # Maps module IDs to names
        self.mod_queue = []  # List of module IDs to track
        self.curr_idx = 0
        self.curr_mod = None

        # Discovery state
        self.finalized = False

        super().__init__(**kwargs)

    def register_mod(self, mod) -> None:
        if self.finalized:
            return
        self.mod_names[id(mod)] = module_name(mod) or mod.__class__.__name__

    def finalize_discovery(self):
        self.finalized = True
        mods = sorted(self.mod_names.items(), key=lambda x: len(x[1]))
        self.mod_queue = [x for x, _ in mods]

        if self.mod_queue:
            self.curr_idx = 0
            self.curr_mod = self.mod_queue[0]

    def should_sample(self, mod) -> bool:
        if not self.finalized:
            self.register_mod(mod)
            return False

        if self.offset() == 0:
            return True

        if self.mode == "ordered":
            return id(mod) == self.curr_mod
        return random.random() < self.rate

    def next_mod(self) -> None:
        if self.mod_queue:
            idx = (self.curr_idx + 1) % len(self.mod_queue)
            self.curr_idx = idx
            self.curr_mod = self.mod_queue[idx]


class PythonTracer:
    def __init__(self, tracepy=False, **kwargs):
        # Set up Python exception tracing if requested
        if tracepy:
            import sys

            sys.settrace(self.trace_exceptions)
        super().__init__(**kwargs)

    def trace_exceptions(self, frame, event, arg):
        """Trace Python exceptions during execution."""
        if event == "exception":
            exception, value, traceback = arg
            if isinstance(value, RuntimeError):
                print(f"Exception: {exception}, Value: {value}")
        return self.trace_exceptions


class MemTracer(BaseTracer, Timer, Sampler, PythonTracer):
    def __init__(self, tracepy=False, sync=False, mode="ordered", rate=0.05):
        self.curr_step = 0
        self.pending = []

        super().__init__(tracepy=tracepy, sync=sync, mode=mode, rate=rate)

    def log_module_stage(self, stage, mod, force=False) -> None:
        # Skip if we shouldn't log this module
        if not force and not self.should_sample(mod):
            return

        record = mem_stats()
        record.step = self.curr_step
        record.seq = self.offset()
        record.module = self.mod_names.get(id(mod), "None")
        record.stage = stage

        if stage.startswith("pre"):
            record.time_offset = self.begin_timing(mod, stage)
            record.save()
        record.time_offset, events = self.end_timing(mod, stage)
        self.pending.append(DelayedRecord(record, events))

    def post_step_hook(self, opt, args, kwargs):
        if not self.finalized:
            self.finalize_discovery()
        else:
            self.curr_step += 1
            self.next_mod()

        # Ensure CUDA operations are complete before processing traces
        if self.has_cuda and self.pending:
            _cuda_sync()

        self.pending = [x for x in self.pending if x.save()]
        self.step_start = 0
        return super().post_step_hook(opt, args, kwargs)


def _cuda_sync():
    import torch

    torch.cuda.synchronize()


def _cuda_event():
    import torch

    event = torch.cuda.Event(enable_timing=True)
    event.record()
    return event
