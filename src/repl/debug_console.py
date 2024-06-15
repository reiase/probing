import code
import io
from contextlib import redirect_stderr, redirect_stdout
from types import CodeType
from typing import Any


def register_debug_command(name, cmd=None):
    if cmd is None:

        def wrapper(cls):
            register_debug_command(name, cls)
            return cls

        return wrapper
    DebugCommand.REGISTER[name] = cmd()


class DebugCommand:
    REGISTER = {}

    def help(self):
        pass

    def __repr__(self) -> str:
        return self()

    def __call__(self, *args: Any, **kwds: Any) -> Any:
        return ""


class _obj_:
    def __init__(self, obj):
        self._obj = obj

    def __repr__(self):
        import json

        return json.dumps(self._obj, indent=2)


def _get_obj_type(obj):
    try:
        m = type(obj).__module__
        n = type(obj).__name__
        return f"{m}.{n}"
    except:
        return str(type(obj))


def _get_obj_repr(obj, value=False):
    typ = _get_obj_type(obj)
    ret = {
        "id": id(obj),
        "class": _get_obj_type(obj),
    }
    if typ == "torch.Tensor":
        ret["shape"] = str(obj.shape)
        ret["dtype"] = str(obj.dtype)
        ret["device"] = str(obj.device)
    if value:
        ret["value"] = str(obj)[:150]
    return ret


@register_debug_command("tprofile")
class TorchHelper(DebugCommand):
    def __call__(self, steps=1):
        TorchHelper.profile(steps)

    @staticmethod
    def get_top_level_modules():
        import gc
        import torch

        objs = gc.get_objects()
        objs = [obj for obj in objs if isinstance(obj, torch.nn.Module)]

        children = set()

        def walk(obj):
            if hasattr(obj, "children"):
                cnt = 0
                for child in obj.children():
                    children.add(id(child))
                    walk(child)
                    cnt += 1
                if cnt == 0:
                    children.add(id(obj))
            else:
                children.add(id(obj))

        for obj in objs:
            walk(obj)
        return [obj for obj in objs if id(obj) not in children]

    def install_profiler(module, steps=1):
        class _profiler:
            def __init__(self, steps) -> None:
                self._steps = steps
                self._profiler = None
                self._count = 0
                self._hooks = []
                self._module = None
                self._status = False

            def install(self, module):
                self._module = module

                import torch

                self._profiler = torch.profiler.profile()
                print(f"installing profiler to module {module}")
                self._hooks.extend(
                    [
                        module.register_forward_pre_hook(self.module_hook),
                    ]
                )

                return self

            def module_hook(self, *args, **kwargs):
                if self._status is False and self._count < self._steps:
                    print("==== start profiling ====")
                    self._profiler.start()
                    self._status = True
                    self._count += 1
                    return
                if self._status is True and self._count >= self._steps:
                    print("==== stop profiling ====")
                    self._profiler.step()
                    self._profiler.stop()
                    self._status = False
                    [h.remove() for h in self._hooks]
                    self._hooks = []
                    TorchHelper.summary()
                    return
                self._count += 1
                self._profiler.step()

            def summary(self):
                if self._profiler is not None:
                    return self._profiler.key_averages(group_by_input_shape=True).table(
                        sort_by="cpu_time_total", row_limit=10
                    )
                else:
                    return "profiler not installed"

        return _profiler(steps).install(module)

    @staticmethod
    def profile(steps=1):
        import __main__

        if not hasattr(__main__, "__probe__"):
            __main__.__probe__ = {}
        tms = TorchHelper.get_top_level_modules()
        for m in tms:
            p = TorchHelper.install_profiler(m, steps)
            if "profiler" not in __main__.__probe__:
                __main__.__probe__["profiler"] = {}
                __main__.__probe__["profiler"][id(m)] = p

    @staticmethod
    def summary():
        import __main__

        if not hasattr(__main__, "__probe__"):
            __main__.__probe__ = {}
        if "profiler" in __main__.__probe__:
            for k, v in __main__.__probe__["profiler"].items():
                print(f"profile for {k}")
                print(v.summary())


@register_debug_command("help")
class HelpCommand(DebugCommand):
    def help(self):
        ret = "list of commands:\n"
        for k, h in DebugCommand.REGISTER.items():
            ret += f"== {k} ==\n"
            if isinstance(h, HelpCommand):
                ret += "print this help"
            else:
                ret += h.help()
            ret += "\n\n"
        return ret

    def __call__(self, *args: Any, **kwds: Any) -> Any:
        return self.help()

    def __repr__(self) -> str:
        return "help command"


@register_debug_command("bt")
class BackTrace(DebugCommand):
    def help(self):
        return "print python and C stack"

    def __call__(self, *args: Any, **kwds: Any) -> Any:
        import traceback

        py = "".join(traceback.format_stack())
        return f"{py}"


@register_debug_command("dump_stack")
class DumpStackCommand(DebugCommand):
    def __call__(self) -> Any:
        stacks = []

        import sys

        curr = sys._getframe(2)
        while curr is not None:
            stack = {
                "file": curr.f_code.co_filename,
                "func": curr.f_code.co_name,
                "lineno": curr.f_lineno,
                "locals": {
                    k: _get_obj_repr(v, value=True) for k, v in curr.f_locals.items()
                },
            }
            stacks.append(stack)
            curr = curr.f_back
        return _obj_(stacks)


@register_debug_command("handle")
class HandleCommand(DebugCommand):
    def parse_query(self, query: str) -> Any:
        import urllib
        import urllib.parse

        return urllib.parse.parse_qs(query)

    def _filter_obj_type(self, obj, type_selector=None, no_builtin=True):
        typ = self._get_obj_type(obj)
        if no_builtin and (
            typ.startswith("builtins.")
            or typ.startswith("codeop")
            or typ.startswith("_io.")
            or typ.startswith("typing.")
            or typ.startswith("_asyncio.")
            or typ.startswith("asyncio.")
            or typ.startswith("six.")
            or typ.startswith("prompt_toolkit.")
            or typ.startswith("_collections.")
            or typ.startswith("_ast.")
            or typ.startswith("ast.")
        ):
            return False
        if type_selector is not None:
            return typ == type_selector
        return True

    def _get_obj_type(self, obj):
        try:
            m = type(obj).__module__
            n = type(obj).__name__
            return f"{m}.{n}"
        except:
            return str(type(obj))

    def _get_obj_repr(self, obj, value=False):
        typ = self._get_obj_type(obj)
        ret = {
            "id": id(obj),
            "class": self._get_obj_type(obj),
        }
        if typ == "torch.Tensor":
            ret["shape"] = str(obj.shape)
            ret["dtype"] = str(obj.dtype)
            ret["device"] = str(obj.device)
        if value:
            ret["value"] = str(obj)
        return ret

    def get_objects(self, type_selector: str = None, limit=None) -> Any:
        limit = int(limit) if limit is not None else None
        import gc
        import json

        class _obj_list_:
            def __init__(self, objs):
                self._objs = objs

            def __repr__(self):
                return json.dumps(self._objs, indent=2)

        objs = gc.get_objects()
        objs = [obj for obj in objs if self._filter_obj_type(obj, type_selector)]
        objs = objs[:limit] if limit is not None else objs
        return _obj_([self._get_obj_repr(obj) for obj in objs])

    def get_torch_tensors(self, limit=None) -> Any:
        import gc

        objs = gc.get_objects()
        objs = [obj for obj in objs if self._filter_obj_type(obj, "torch.Tensor")]
        objs = objs[: int(limit)] if limit is not None else objs
        return _obj_([self._get_obj_repr(obj) for obj in objs])

    def get_torch_modules(self, limit=None, toplevel=None) -> Any:
        limit = int(limit) if limit is not None else None
        toplevel = toplevel in ["true", "True", "T"] if toplevel is not None else False
        import gc
        import torch

        objs = gc.get_objects()
        objs = [obj for obj in objs if isinstance(obj, torch.nn.Module)]

        children = set()

        def walk(obj):
            if hasattr(obj, "children"):
                for child in obj.children():
                    children.add(id(child))
                    walk(child)

        for obj in objs:
            walk(obj)
        if toplevel:
            objs = [obj for obj in objs if id(obj) not in children]

        objs = objs[: int(limit)] if limit is not None else objs
        return _obj_([self._get_obj_repr(obj, value=True) for obj in objs])

    def __call__(self, path=None, query=None) -> Any:
        params = self.parse_query(query) if query is not None else {}
        if path == "/objects":
            return self.get_objects(
                type_selector=params.get("type", [None])[0],
                limit=params.get("limit", [None])[0],
            )
        if path == "/torch/tensors":
            return self.get_torch_tensors(
                limit=params.get("limit", [None])[0],
            )
        if path == "/torch/modules":
            return self.get_torch_modules(
                limit=params.get("limit", [None])[0],
            )
        return None


class DebugConsole(code.InteractiveConsole):
    def init(self):
        for k, v in DebugCommand.REGISTER.items():
            self.locals[k] = v()

    def resetoutput(self):
        out = self.output
        self.output = ""
        return out

    def write(self, data: str) -> None:
        self.output += data

    def runsource(
        self, source: str, filename: str = "<input>", symbol: str = "single"
    ) -> bool:
        try:
            code = self.compile(source, filename, symbol)
        except (OverflowError, SyntaxError, ValueError):
            # Case 1: wrong code
            self.showsyntaxerror(filename)
            self.resetbuffer()
            return self.resetoutput()

        if code is None:
            # Case 2: incomplete code
            return

        ret = self.runcode(code)
        self.resetbuffer()
        return ret

    def runcode(self, code: CodeType) -> None:
        # try:
        #     with redirect_stderr(io.StringIO()) as err:
        #         with redirect_stdout(io.StringIO()) as out:
        #             exec(code, self.locals, DebugCommand.REGISTER)
        #     ret = err.getvalue() + out.getvalue()
        #     if len(ret) == 0:
        #         return None
        #     return ret

        # except SystemExit:
        #     raise
        # except:
        #     self.showtraceback()
        #     return self.resetoutput()

        with redirect_stderr(io.StringIO()) as err:
            with redirect_stdout(io.StringIO()) as out:
                try:
                    exec(code, self.locals, DebugCommand.REGISTER)
                except SystemExit:
                    raise
                except:
                    ret = err.getvalue() + out.getvalue()
                    self.showtraceback()
                    return ret + self.resetoutput()
        ret = out.getvalue()
        if len(ret) == 0:
            return None
        return ret

    def push(self, line: str) -> bool:
        if not hasattr(self, "output"):
            self.output = ""
        self.buffer.append(line)
        source = "\n".join(self.buffer)
        return self.runsource(source, self.filename)


debug_console = DebugConsole()
globals()
