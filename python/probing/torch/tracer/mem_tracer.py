import time
from dataclasses import dataclass

from .types import BaseTracer
from .module_utils import module_name
from ..step import next_step, step

from probing.table import table
import random


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


def get_mem_info():
    import torch

    return {
        "allocated": torch.cuda.memory_allocated() / 1024 / 1024,
        "cached": torch.cuda.memory_reserved() / 1024 / 1024,
        "max_allocated": torch.cuda.max_memory_allocated() / 1024 / 1024,
        "max_cached": torch.cuda.max_memory_reserved() / 1024 / 1024,
    }


OFFSET = 0


class MemTracer(BaseTracer):
    def __init__(self, tracepy=False, logtime=False, sync=False, sample_rate=0.05):
        self.logtime = logtime
        self.need_wait = sync
        self.sample_rate = sample_rate
        if tracepy:
            import sys

            sys.settrace(self.pytrace)
        super().__init__()

    def pytrace(self, frame, event, arg):
        if event == "exception":
            exception, value, traceback = arg
            if isinstance(value, RuntimeError):
                print(f"Exception: {exception}, Value: {value}", file=self.logfile)
        return self.pytrace

    def log(self, stage, m):
        if random.random() > self.sample_rate:
            return
        import torch

        global OFFSET
        ts = 0

        if self.logtime:
            if self.need_wait:
                torch.cuda.synchronize()
            ts = time.time()

        mem = get_mem_info()

        TorchTrace(
            step=step(),
            offset=OFFSET,
            module=module_name(m) or "None",
            stage=stage,
            allocated=mem["allocated"],
            max_allocated=mem["max_allocated"],
            cached=mem["cached"],
            max_cached=mem["max_cached"],
            time=ts,
        ).save()

    def pre_forward_hook(self, m, i):
        global OFFSET
        OFFSET += 1

        self.log("pre forward", m)
        return super().pre_forward_hook(m, i)

    def post_forward_hook(self, m, i, o):
        global OFFSET
        OFFSET += 1

        self.log("post forward", m)
        return super().post_forward_hook(m, i, o)

    def pre_backward_hook(self, m, i):
        global OFFSET
        OFFSET += 1

        self.log("pre backward", m)
        return super().pre_backward_hook(m, i)

    def post_backward_hook(self, m, i, o):
        global OFFSET
        OFFSET += 1

        self.log("post backward", m)
        return super().post_backward_hook(m, i, o)

    def pre_step_hook(self, optimizer, args, kwargs):
        global OFFSET
        OFFSET += 1

        self.log("pre step", optimizer)
        return super().pre_step_hook(optimizer, args, kwargs)

    def post_step_hook(self, optimizer, args, kwargs):
        global OFFSET
        OFFSET += 1

        self.log("post step", optimizer)
        next_step()

        OFFSET = 0
        return super().post_step_hook(optimizer, args, kwargs)
