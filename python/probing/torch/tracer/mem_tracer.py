import time
from dataclasses import dataclass
from typing import Dict, Optional
import random

from .types import BaseTracer
from .module_utils import module_name
from probing.table import table


@table("torch_trace")
@dataclass
class TorchTrace:
    """
    Records memory and timing information for PyTorch module operations.

    Attributes:
        step: Current training step number
        offset: Operation offset within the step
        module: Module name being traced
        stage: Current operation stage (pre_forward, post_forward, etc.)
        allocated: Currently allocated memory in MB
        max_allocated: Peak allocated memory in MB
        cached: Currently cached memory in MB
        max_cached: Peak cached memory in MB
        time: Timestamp when trace was captured
        duration: Duration of the operation in seconds
    """

    step: int
    offset: int
    module: str
    stage: str
    allocated: float  # Using float for better precision
    max_allocated: float
    cached: float
    max_cached: float
    time: float
    duration: float = 0.0


def get_memory_stats() -> Dict[str, float]:
    """
    Get current GPU memory statistics in MB.

    Returns:
        Dictionary containing memory metrics:
        - allocated: Currently allocated memory
        - cached: Currently cached/reserved memory
        - max_allocated: Peak allocated memory
        - max_cached: Peak reserved memory
    """
    import torch

    MB = 1024 * 1024
    return {
        "allocated": torch.cuda.memory_allocated() / MB,
        "cached": torch.cuda.memory_reserved() / MB,
        "max_allocated": torch.cuda.max_memory_allocated() / MB,
        "max_cached": torch.cuda.max_memory_reserved() / MB,
    }


class ModuleTimer:
    """
    Handles precise timing for module operations during forward/backward pass.

    Uses both CPU timers and CUDA events (when available) to measure execution time
    of module operations with high accuracy.
    """

    def __init__(self, logtime: bool = False, sync: bool = False, **kwargs):
        """
        Initialize the timer.

        Args:
            logtime: Whether to perform timing measurements
            sync: Whether to synchronize CUDA before timing
            **kwargs: Additional arguments passed to parent classes
        """
        import torch

        self.has_cuda = torch.cuda.is_available()
        self.logtime = logtime
        self.sync = sync
        self.start_times = {}  # CPU timers
        self.cuda_events = {}  # GPU timers

        super().__init__(**kwargs)

    def begin_timing(self, module, stage) -> Optional[float]:
        """
        Start timing for a specific module and stage.

        Args:
            module: The PyTorch module being timed
            stage: Operation stage (e.g. 'pre_forward')

        Returns:
            Current timestamp or None if timing is disabled
        """
        if not self.logtime:
            return None

        module_id = id(module)
        key = (module_id, stage)

        # Synchronize if needed for more accurate timing
        if self.sync and self.has_cuda:
            import torch

            torch.cuda.synchronize()

        # Record CPU time
        timestamp = time.time()
        self.start_times[key] = timestamp

        # Create CUDA event if available
        if self.has_cuda:
            import torch

            start_event = torch.cuda.Event(enable_timing=True)
            start_event.record()
            self.cuda_events[key] = start_event
        return timestamp

    def end_timing(self, module, stage) -> float:
        """End timing for a specific module and stage, returning duration in seconds."""
        if not self.logtime:
            return 0.0

        module_id = id(module)
        key = (module_id, stage)

        timestamp = time.time()
        duration = 0.0

        if key in self.cuda_events:
            import torch

            end_event = torch.cuda.Event(enable_timing=True)
            end_event.record()

            if self.sync:
                torch.cuda.synchronize()

            duration = self.cuda_events[key].elapsed_time(end_event) / 1000.0
            del self.cuda_events[key]
            del self.start_times[key]
        elif key in self.start_times:
            duration = timestamp - self.start_times[key]
            del self.start_times[key]

        return timestamp, duration


class SamplingStrategy:
    """Defines strategies for sampling modules during memory tracing."""

    def __init__(self, strategy="ordered", sample_rate=0.05, **kwargs):
        # Strategy configuration
        self.strategy = strategy
        self.sample_rate = sample_rate

        # Module tracking state
        self.module_names = {}  # Maps module IDs to names
        self.module_ids = []  # List of module IDs to track
        self.curr_module_idx = 0
        self.tracking_module = None

        # Discovery state
        self.discovery_done = False

        super().__init__(**kwargs)

    def register_new_module(self, module) -> None:
        """Register a newly discovered module during the discovery phase."""
        if self.discovery_done:
            return

        module_id = id(module)
        if module_id not in self.module_names:
            import torch

            self.module_names[module_id] = module_name(module) or (
                "None"
                if isinstance(module, torch.nn.Module)
                else module.__class__.__name__
            )

    def complete_discovery(self):
        """Complete the module discovery phase and prepare for sampling."""
        self.discovery_done = True

        modules = sorted(self.module_names.items(), key=lambda x: len(x[1]))
        self.module_ids = [module_id for module_id, _ in modules]

        if self.module_ids:
            self.tracking_module = self.module_ids[0]

    def should_log_module(self, module) -> bool:
        """Determine if the current module should be logged based on sampling strategy."""
        if not self.discovery_done:
            self.register_new_module(module)
            return False

        if self.offset() == 0:
            return True

        if self.strategy == "ordered":
            return id(module) == self.tracking_module
        return random.random() < self.sample_rate

    def select_next_module(self) -> None:
        """Select the next module to track in round-robin fashion."""
        if self.module_ids:
            idx = (self.curr_module_idx + 1) % len(self.module_ids)
            self.curr_module_idx = idx
            self.tracking_module = self.module_ids[idx]


class PythonTracer:
    def __init__(self, tracepy=False):
        # Set up Python exception tracing if requested
        if tracepy:
            import sys

            sys.settrace(self.trace_exceptions)

    def trace_exceptions(self, frame, event, arg):
        """Trace Python exceptions during execution."""
        if event == "exception":
            exception, value, traceback = arg
            if isinstance(value, RuntimeError):
                print(f"Exception: {exception}, Value: {value}")
        return self.trace_exceptions


class MemTracer(BaseTracer, ModuleTimer, SamplingStrategy, PythonTracer):
    """
    Memory tracer for PyTorch modules that samples one module per step.

    This tracer discovers modules during the first training step, then
    cycles through them in subsequent steps, sampling from outer modules
    (shorter names) to inner modules (longer names).
    """

    def __init__(
        self,
        tracepy=False,
        logtime=False,
        sync=False,
        strategy="ordered",
        sample_rate=0.05,
    ):
        # Current step counter
        self.current_step = 0

        # Initialize parent classes
        super().__init__(
            tracepy=tracepy,
            logtime=logtime,
            sync=sync,
            strategy=strategy,
            sample_rate=sample_rate,
        )

    def log_module_stage(self, stage, module, force=False) -> None:
        """Record memory usage for the given module and stage."""
        # Skip if we shouldn't log this module
        if not force and not self.should_log_module(module):
            return

        if self.logtime:
            # Synchronize CUDA if needed
            if self.sync and self.has_cuda:
                import torch

                torch.cuda.synchronize()

            if stage.startswith("pre"):
                timestamp, duration = self.begin_timing(module, stage), 0.0
            elif stage.startswith("post"):
                timestamp, duration = self.end_timing(module, stage)
        else:
            timestamp, duration = 0, 0

        # Get and record memory statistics
        memory_stats = get_memory_stats()

        TorchTrace(
            step=self.current_step,
            offset=self.offset(),
            module=self.module_names.get(id(module), "None"),
            stage=stage,
            allocated=memory_stats["allocated"],
            max_allocated=memory_stats["max_allocated"],
            cached=memory_stats["cached"],
            max_cached=memory_stats["max_cached"],
            time=timestamp,
            duration=duration,
        ).save()

    def post_step_hook(self, optimizer, args, kwargs):
        """
        Process actions after each optimization step:
        - First step: Complete module discovery
        - Later steps: Select next module to track
        """

        if not self.discovery_done:
            self.complete_discovery()
        else:
            self.current_step += 1
            self.select_next_module()
        return super().post_step_hook(optimizer, args, kwargs)
