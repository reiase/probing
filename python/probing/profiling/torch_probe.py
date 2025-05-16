import random
import time
from dataclasses import dataclass
from typing import Optional

from probing.core import table

from .torch.module_utils import module_name
from .types import BaseTracer


@table
@dataclass
class TorchTrace:
    step: Optional[int] = None
    seq: Optional[int] = None
    module: Optional[str] = None
    stage: Optional[str] = None
    allocated: float = 0.0
    max_allocated: float = 0.0
    cached: float = 0.0
    max_cached: float = 0.0
    time_offset: float = 0.0
    duration: float = 0.0
    duration: float = 0.0


@table
@dataclass
class Variables:
    step: Optional[int] = None
    func: Optional[str] = None
    name: Optional[str] = None
    value: Optional[str] = None


class DelayedRecord:
    def __init__(self, record, events):
        self.record = record
        self.events = events

    def save(self):
        try:
            if self.events is not None:
                start, end = self.events
                self.record.duration = start.elapsed_time(end) / 1000.0
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


STAGEMAP = {
    "pre forward": "forward",
    "post forward": "forward",
    "pre backward": "backward",
    "post backward": "backward",
    "pre step": "step",
    "post step": "step",
}


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
            key = (id(mod), STAGEMAP[stage])
            self.events[key] = _cuda_event()
        return time_offset

    def end_timing(self, mod, stage) -> tuple:
        # Synchronize if needed for more accurate timing
        if self.sync and self.has_cuda:
            _cuda_sync()

        time_offset = time.time() - self.step_start
        key = (id(mod), STAGEMAP[stage])

        if key in self.events:
            return time_offset, (self.events.pop(key), _cuda_event())
        return time_offset, None


class Sampler:
    def __init__(self, mode="ordered", rate=1.0, **kwargs):
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
        self.sampled_step = True

        super().__init__(**kwargs)

    def register_mod(self, mod) -> None:
        if self.finalized:
            return

        import torch

        self.mod_names[id(mod)] = module_name(mod) or (
            mod.__class__.__name__ if isinstance(mod, torch.optim.Optimizer) else "None"
        )

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

        if not self.sampled_step:
            return False

        if self.offset() == 0:
            return True

        if self.mode == "ordered":
            return id(mod) == self.curr_mod
        return random.random() < self.rate

    def next_mod(self) -> None:
        if self.mod_queue and self.mode == "ordered":
            self.sampled_step = random.random() < self.rate
            idx = (self.curr_idx + 1) % len(self.mod_queue)
            self.curr_idx = idx
            self.curr_mod = self.mod_queue[idx]

    def set_sampling_mode(self, expr):
        """Set the sampling mode and rate based on the provided expression.

        The expression should be in the format "mode:rate", where mode can be
        "ordered" or "random", and rate is a float between 0 and 1.

        Examples
        --------

        >>> tracer = TorchProbe()
        >>> tracer.mode, tracer.rate
        ('ordered', 1.0)

        >>> tracer.set_sampling("random:0.1")
        >>> tracer.mode, tracer.rate
        ('random', 0.1)

        >>> tracer.set_sampling("ordered:0.5")
        >>> tracer.mode, tracer.rate
        ('ordered', 0.5)

        >>> tracer.set_sampling("invalid:1.5")
        >>> tracer.mode, tracer.rate
        ('ordered', 1.0)
        """
        if expr == "ordered":
            self.mode = "ordered"
            self.rate = 1.0
            return
        try:
            mode, rate = expr.split(":")

            self.mode = mode if mode in ["ordered", "random"] else "ordered"
            self.rate = float(rate) if 0 < float(rate) <= 1 else 1.0
        except ValueError:
            print(f"Invalid sampling expression: {expr}. Using default settings.")
            self.mode = "ordered"
            self.rate = 1.0


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


class VariableTracer:
    """
    Traces specified variables within functions during execution.

    This class allows you to monitor variables in specific functions by providing
    expressions in the format "variable@function". When the traced functions are
    executed, the class captures the variable values and saves them.

    Parameters:
        exprs (str): Comma-separated list of expressions in format "var@func"
                    where 'var' is the variable name and 'func' is the function name.
        **kwargs: Additional keyword arguments passed to parent classes.

    Examples:
        >>> # Simple initialization with one variable in one function
        >>> tracer = VariableTracer("x@calculate")
        >>> tracer.variabls
        {'calculate': ['x']}

        >>> # Multiple variables in different functions
        >>> tracer = VariableTracer("x@calculate,y@process,z@calculate")
        >>> sorted(tracer.variabls.keys())
        ['calculate', 'process']
        >>> sorted(tracer.variabls['calculate'])
        ['x', 'z']
        >>> tracer.variabls['process']
        ['y']

        >>> # Empty string initialization
        >>> tracer = VariableTracer("")
        >>> tracer.variabls
        {}

        >>> # Handling whitespace
        >>> tracer = VariableTracer(" a@func1 , b@func2 ")
        >>> tracer.variabls
        {'func1': ['a'], 'func2': ['b']}
    """

    def __init__(self, exprs="", **kwargs):
        self.variabls = {}
        for expr in exprs.split(","):  # Fixed: using exprs instead of expr
            expr = expr.strip()
            if "@" in expr:
                var, fun = expr.split("@")
                if fun not in self.variabls:
                    self.variabls[fun] = []
                self.variabls[fun].append(var)

    def trace_variables(self):
        """
        Traces variables specified during initialization in the current execution stack.

        This method inspects the call stack, looking for functions specified during
        initialization. When found, it retrieves the values of the specified variables
        and saves them using the Variables dataclass.

        Note: This method requires access to self.curr_step which should be set by
        a parent class.
        """
        if not self.variabls:
            return

        import inspect

        stacks = inspect.stack()[1:]
        for stack in stacks:
            frame = stack.frame
            code = frame.f_code
            func = code.co_name
            if func in self.variabls:
                for var in self.variabls[func]:
                    if var in frame.f_locals:
                        val = frame.f_locals[var]
                        try:
                            val = str(val)
                        except Exception as e:
                            val = f"{type(val)}"
                        Variables(self.curr_step, func, var, val).save()


class TorchProbe(BaseTracer, Timer, Sampler, PythonTracer, VariableTracer):
    def __init__(self, tracepy=False, sync=False, mode="ordered", rate=1.0, exprs=""):
        self.curr_step = 0
        self.pending = []

        super().__init__(tracepy=tracepy, sync=sync, mode=mode, rate=rate, exprs=exprs)

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
            # record.save()
            self.pending.append(DelayedRecord(record, None))
        else:
            record.time_offset, events = self.end_timing(mod, stage)
            self.pending.append(DelayedRecord(record, events))

    def post_step_hook(self, opt, args, kwargs):
        super().post_step_hook(opt, args, kwargs)
        if not self.finalized:
            self.finalize_discovery()
        else:
            self.curr_step += 1
            self.next_mod()

        # Ensure CUDA operations are complete before processing traces
        if self.has_cuda and self.pending:
            _cuda_sync()

        # process pending records
        self.pending = [x for x in self.pending if x.save()]

        # trace Python variables
        self.trace_variables()

        # reset the step start time
        self.step_start = 0


def _cuda_sync():
    import torch

    torch.cuda.synchronize()


def _cuda_event():
    import torch

    event = torch.cuda.Event(enable_timing=True)
    event.record()
    return event


def set_sampling_mode(mode):
    import gc

    objs = [obj for obj in gc.get_objects() if isinstance(obj, TorchProbe)]
    try:
        for obj in objs:
            obj.set_sampling_mode(mode)
    except Exception as e:
        print(f"Error setting mode: {e}")
