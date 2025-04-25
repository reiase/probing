import time
from dataclasses import dataclass

from .types import BaseTracer
from .module_utils import module_name
from ..step import step

from probing.table import table


@table("torch_trace")
@dataclass
class TorchTrace:
    step: int
    offset: int
    module: str
    stage: str
    allocated: int
    max_allocated: int
    cached: int
    max_cached: int
    time: float


def get_memory_stats():
    """Get current GPU memory statistics in MB"""

    import torch

    return {
        "allocated": torch.cuda.memory_allocated() / 1024 / 1024,
        "cached": torch.cuda.memory_reserved() / 1024 / 1024,
        "max_allocated": torch.cuda.max_memory_allocated() / 1024 / 1024,
        "max_cached": torch.cuda.max_memory_reserved() / 1024 / 1024,
    }


class MemTracer(BaseTracer):
    """
    Memory tracer for PyTorch modules that samples one module per step.

    This tracer discovers modules during the first training step, then
    cycles through them in subsequent steps, sampling from outer modules
    (shorter names) to inner modules (longer names).
    """

    def __init__(self, tracepy=False, logtime=False, sync=False):
        self.logtime = logtime
        self.sync = sync

        # Dictionary mapping module IDs to their names
        self.module_names = {}

        # List of module IDs in order of sampling priority
        self.module_ids = []

        # Current position in the sampling order
        self.curr_module_idx = 0
        self.tracking_module = None

        # State tracking flags
        self.discovery_done = False
        self.current_step = 0

        if tracepy:
            import sys

            sys.settrace(self.trace_exceptions)
        super().__init__()

    def trace_exceptions(self, frame, event, arg):
        """Trace Python exceptions during execution"""
        if event == "exception":
            exception, value, traceback = arg
            if isinstance(value, RuntimeError):
                print(f"Exception: {exception}, Value: {value}")
        return self.trace_exceptions

    def register_new_module(self, module):
        """Register a newly discovered module during the discovery phase"""
        # Skip registration if discovery phase is already complete
        if self.discovery_done:
            return

        module_id = id(module)
        if module_id not in self.module_names:
            self.module_names[module_id] = module_name(module) or "None"

    def should_log_module(self, module):
        """Determine if this module should be logged for the current step"""

        if not self.discovery_done:
            self.register_new_module(module)
            return False

        module_id = id(module)

        if module_id == self.tracking_module:
            return True
        return False

    def log_module_stage(self, stage, module, force=False):
        """Record memory usage for the given module and stage"""
        # Skip if we shouldn't log this module
        if not force and not self.should_log_module(module):
            return

        global MODULE_CALL_OFFSET
        timestamp = 0

        # Get timing information if requested
        if self.logtime:
            if self.sync:
                import torch

                torch.cuda.synchronize()
            timestamp = time.time()

        # Get memory usage statistics
        memory_stats = get_memory_stats()

        # Save the trace data
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
        ).save()

    def post_step_hook(self, optimizer, args, kwargs):
        """
        Process after each optimization step:
        - First step: Complete discovery and sort modules by name length
        - Later steps: Cycle through modules to track memory usage
        """
        # Handle first step (discovery phase)
        if not self.discovery_done:
            self.discovery_done = True
            
            # Sort modules by name length (shorter names first)
            mods = list(self.module_names.items())
            mods.sort(key=lambda x: len(x[1]))
            self.module_ids = [mid for mid, _ in mods]
            return super().post_step_hook(optimizer, args, kwargs)

        # Handle subsequent steps - cycle to next module
        self.current_step += 1
        if self.module_ids:
            # Select next module to track in round-robin fashion
            idx = self.curr_module_idx
            idx = (idx + 1) % len(self.module_ids)
            self.curr_module_idx = idx
            self.tracking_module = self.module_ids[idx]
            
        return super().post_step_hook(optimizer, args, kwargs)
