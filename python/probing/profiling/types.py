from dataclasses import dataclass
from typing import Any, List

from .torch.step import next_step


@dataclass
class TensorDef:
    shape: tuple = ()
    dtype: Any = None

    @staticmethod
    def create(t):
        return TensorDef(t.shape, t.dtype)


# Global counter for tracking execution order within a step
MODULE_CALL_OFFSET = 0
CURRENT_MODULE = None
CURRENT_STAGE = None


class BaseTracer:
    def offset(self):
        return MODULE_CALL_OFFSET

    def process_hook(self, module, stage):
        global MODULE_CALL_OFFSET
        global CURRENT_MODULE
        global CURRENT_STAGE

        if CURRENT_MODULE != id(module) or CURRENT_STAGE != stage:
            MODULE_CALL_OFFSET += 1
            CURRENT_MODULE = id(module)
            CURRENT_STAGE = stage

    def pre_forward_hook(self, m, i):
        self.log_module_stage("pre forward", m)
        self.process_hook(m, "pre forward")

    def post_forward_hook(self, m, i, o):
        self.log_module_stage("post forward", m)
        self.process_hook(m, "post forward")

    def pre_backward_hook(self, m, i):
        self.log_module_stage("pre backward", m)
        self.process_hook(m, "pre backward")

    def post_backward_hook(self, m, i, o):
        self.log_module_stage("post backward", m)
        self.process_hook(m, "post backward")

    def pre_step_hook(self, optimizer, args, kwargs):
        self.log_module_stage("pre step", optimizer, force=False)
        self.process_hook(optimizer, "pre step")

    def post_step_hook(self, optimizer, args, kwargs):
        self.log_module_stage("post step", optimizer, force=False)
        self.process_hook(optimizer, "post step")
        global MODULE_CALL_OFFSET
        MODULE_CALL_OFFSET = 0
        next_step()
