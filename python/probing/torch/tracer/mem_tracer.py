import time
from dataclasses import dataclass

from .types import BaseTracer
from .module_utils import module_name
from ..step import step

from probing.table import table


@table
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


# Global counter for tracking execution order within a step
MODULE_CALL_OFFSET = 0


class MemTracer(BaseTracer):
    """
    Memory tracer for PyTorch modules that samples one module per step.
    
    This tracer discovers modules during the first training step, then
    cycles through them in subsequent steps, sampling from outer modules
    (shorter names) to inner modules (longer names).
    """
    
    def __init__(self, tracepy=False, logtime=False, sync=False):
        self.logtime = logtime
        self.should_synchronize = sync

        # Dictionary mapping module IDs to their names
        self.module_name_map = {}
        
        # List of module IDs in order of sampling priority
        self.sampling_order = []

        # Current position in the sampling order
        self.current_module_index = 0

        # State tracking flags
        self.discovery_phase_complete = False
        self.logged_in_current_step = False
        self.current_step_number = 0

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
        if self.discovery_phase_complete:
            return
            
        module_id = id(module)
        if module_id not in self.module_name_map:
            # Store the module name
            self.module_name_map[module_id] = module_name(module) or "None"
            
            # # Add to sampling order (will be properly sorted at end of discovery phase)
            # self.sampling_order.append(module_id)

    def should_log_module(self, module):
        """Determine if this module should be logged for the current step"""
        # Always attempt to register new modules during discovery phase
        if not self.discovery_phase_complete:
            self.register_new_module(module)
        
        # Safety check for empty sampling order
        if not self.sampling_order:
            return False
        
        # Check if we've moved to a new step
        current_step_value = step()
        if current_step_value != self.current_step_number:
            # Reset for new step
            self.current_step_number = current_step_value
            self.logged_in_current_step = False

            # Advance to next module in sampling order
            if self.sampling_order:
                self.current_module_index = (self.current_module_index + 1) % len(self.sampling_order)

        # Skip if we've already logged something in this step
        if self.logged_in_current_step:
            return False

        # Check if this is the currently selected module for sampling
        return id(module) == self.sampling_order[self.current_module_index]

    def log_module_stats(self, stage, module):
        """Record memory usage for the given module and stage"""
        # Skip if we shouldn't log this module
        if not self.should_log_module(module):
            return

        global MODULE_CALL_OFFSET
        timestamp = 0

        # Get timing information if requested
        if self.logtime:
            if self.should_synchronize:
                import torch

                torch.cuda.synchronize()
            timestamp = time.time()

        # Get memory usage statistics
        memory_stats = get_memory_stats()

        # Save the trace data
        TorchTrace(
            step=self.current_step_number,
            offset=MODULE_CALL_OFFSET,
            module=self.module_name_map.get(id(module), "None"),
            stage=stage,
            allocated=memory_stats["allocated"],
            max_allocated=memory_stats["max_allocated"],
            cached=memory_stats["cached"],
            max_cached=memory_stats["max_cached"],
            time=timestamp,
        ).save()
        
        # Mark that we've logged for this step
        self.logged_in_current_step = True

    def pre_forward_hook(self, m, i):
        global MODULE_CALL_OFFSET
        MODULE_CALL_OFFSET += 1
        self.log_module_stats("pre forward", m)
        return super().pre_forward_hook(m, i)

    def post_forward_hook(self, m, i, o):
        global MODULE_CALL_OFFSET
        MODULE_CALL_OFFSET += 1
        self.log_module_stats("post forward", m)
        return super().post_forward_hook(m, i, o)

    def pre_backward_hook(self, m, i):
        global MODULE_CALL_OFFSET
        MODULE_CALL_OFFSET += 1
        self.log_module_stats("pre backward", m)
        return super().pre_backward_hook(m, i)

    def post_backward_hook(self, m, i, o):
        global MODULE_CALL_OFFSET
        MODULE_CALL_OFFSET += 1
        self.log_module_stats("post backward", m)
        return super().post_backward_hook(m, i, o)

    def pre_step_hook(self, optimizer, args, kwargs):
        global MODULE_CALL_OFFSET
        MODULE_CALL_OFFSET += 1
        self.log_module_stats("pre step", optimizer)
        return super().pre_step_hook(optimizer, args, kwargs)

    def post_step_hook(self, optimizer, args, kwargs):
        global MODULE_CALL_OFFSET
        MODULE_CALL_OFFSET += 1
        self.log_module_stats("post step", optimizer)
        
        # Complete discovery phase after first step
        if not self.discovery_phase_complete:
            self.discovery_phase_complete = True
            
            modules = list(self.module_name_map.items())
            modules.sort(key=lambda item: len(item[1]))
            self.sampling_order = [mid for mid, _ in modules]
            print(f"Module discovery complete. Found {len(self.sampling_order)} modules to track.")
        
        MODULE_CALL_OFFSET = 0
        return super().post_step_hook(optimizer, args, kwargs)