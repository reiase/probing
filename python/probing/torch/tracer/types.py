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


class BaseTracer:
    def pre_forward_hook(self, m, i):
        pass

    def post_forward_hook(self, m, i, o):
        pass

    def pre_backward_hook(self, m, i):
        pass

    def post_backward_hook(self, m, i, o):
        pass

    def pre_step_hook(self, optimizer, args, kwargs):
        pass

    def post_step_hook(self, optimizer, args, kwargs):
        next_step()
