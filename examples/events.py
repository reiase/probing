import ast
from dataclasses import dataclass
from typing import Any

import torch


def _get_fullname(m):
    return f"{m.__module__}.{m.__class__.__name__}"


@dataclass
class TenserDef:
    shape: tuple = ()
    dtype: Any = None

    def __repr__(self):
        return f"TenserDef({self.shape}, {self.dtype})"


class Event(dict):
    """
    Examples
    --------
    >>> event = Event()
    >>> event.name = "event name"
    >>> event.name
    'event name'
    """

    def __getattr__(self, name):
        if name in self:
            return self[name]
        else:
            raise AttributeError(f"'{self.__class__.__name__}' object has no attribute '{name}'")

    def __setattr__(self, name: str, value: Any) -> None:
        return self.__setitem__(name, value)


class TorchEvent(Event):
    def __init__(self, name, module, inputs, params={}):
        self.name = name
        self.module = _get_fullname(module)
        try:
            if isinstance(inputs, torch.Tensor):
                self.inputs = [TenserDef(tuple(inputs.shape), inputs.dtype)]
            else:
                self.inputs = [TenserDef(tuple(x.shape), x.dtype) for x in inputs]
        except:
            self.inputs = None
        self.params = params

    def __repr__(self) -> str:
        return f'{self.name}("{self.module}", {self.inputs}, {self.params})'


class ForwardStartEvent(TorchEvent):
    def __init__(self, m, inputs, params={}) -> None:
        super().__init__("ForwardStartEvent", m, inputs, params=params)


class ForwardEndEvent(TorchEvent):
    def __init__(self, m, outputs, params={}) -> None:
        super().__init__("ForwardEndEvent", m, outputs, params=params)


class BackwardStartEvent(TorchEvent):
    def __init__(self, m, inputs, params={}) -> None:
        super().__init__("BackwardStartEvent", m, inputs, params=params)


class BackwardEndEvent(TorchEvent):
    def __init__(self, m, outputs, params={}) -> None:
        super().__init__("BackwardEndEvent", m, outputs, params=params)


def parse(line):
    expr = ast.parse(line, mode="eval")
    name = expr.body.func.id
    args = [eval(compile(ast.Expression(body=arg), filename="", mode="eval")) for arg in expr.body.args]
    event = Event()
    event.name = name
    event.module = args[0]
    if name == "SpanStartEvent":
        event.id = args[1]
    elif name == "SpanEndEvent":
        event.id = args[1]
        event.duration = args[2]
    else:
        event.inputs = args[1]
        event.params = args[2] if len(args) > 2 else {}
    return event


def parse_tree(lines):
    tree = []

    span_stack = []
    curr_span = None

    event_stack = []
    for line in lines:
        event = parse(line)
        if event.name == "SpanStartEvent":
            new_span = Event()
            if curr_span is not None:
                curr_span.children.append(new_span)
            curr_span = new_span
            curr_span.name = "Span"
            curr_span.module = event.module
            curr_span.id = event.id
            curr_span.duration = None
            curr_span.children = []
            tree.append(curr_span)
            span_stack.append(curr_span)
        if event.name == "SpanEndEvent":
            curr_span.duration = event.duration
            curr_span.children = curr_span.children
            curr_span = span_stack.pop()

        if event.name == "ForwardStartEvent":
            if curr_span is not None:
                curr_span.children.append(event)
                event_stack.append(event)

        if event.name == "ForwardEndEvent":
            if len(event_stack) > 0:
                curr_event = event_stack.pop()
                curr_event.outputs = event.inputs[0]

    return tree
