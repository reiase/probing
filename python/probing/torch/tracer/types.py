from typing import Any, List
from dataclasses import dataclass

from ..step import next_step


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
        self.process_hook(m, "pre forward")
        self.log_module_stage("pre forward", m)

    def post_forward_hook(self, m, i, o):
        self.process_hook(m, "post forward")
        self.log_module_stage("post forward", m)

    def pre_backward_hook(self, m, i):
        self.process_hook(m, "pre backward")
        self.log_module_stage("pre backward", m)

    def post_backward_hook(self, m, i, o):
        self.process_hook(m, "post backward")
        self.log_module_stage("post backward", m)

    def pre_step_hook(self, optimizer, args, kwargs):
        self.process_hook(optimizer, "pre step")
        self.log_module_stage("pre step", optimizer, force=True)

    def post_step_hook(self, optimizer, args, kwargs):
        self.process_hook(optimizer, "post step")
        self.log_module_stage("post step", optimizer, force=True)

        MODULE_CALL_OFFSET = 0
        next_step()
