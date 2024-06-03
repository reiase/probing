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


@register_debug_command("params")
class ParamsCommand(DebugCommand):
    def help(self):
        return "list of parameters"

    def __call__(self) -> Any:
        import json

        try:
            from hyperparameter import param_scope

            params = param_scope().storage().storage()
            return json.dumps(params)
        except:
            return ""


@register_debug_command("objects")
class ObjectsCommand(DebugCommand):
    def help(self):
        return "list of objects"

    def _get_obj_type(self, obj):
        try:
            m = type(obj).__module__
            n = type(obj).__name__
            return f"{m}.{n}"
        except:
            return str(type(obj))

    def _get_obj_repr(self, obj):
        typ = self._get_obj_type(obj)
        ret = {
            "id": id(obj),
            "type": self._get_obj_type(obj),
        }
        if typ == "torch.Tensor":
            ret["shape"] = obj.shape
            ret["dtype"] = str(obj.dtype)
            ret["device"] = str(obj.device)
        return ret

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

    def __call__(self, type_selector: str = None, limit=None) -> Any:
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
        return _obj_list_([self._get_obj_repr(obj) for obj in objs])


@register_debug_command("exit")
class ExitCommand(DebugCommand):
    def help(self):
        return "exit debug server"


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
        try:
            with redirect_stderr(io.StringIO()) as err:
                with redirect_stdout(io.StringIO()) as out:
                    exec(code, self.locals, DebugCommand.REGISTER)
            ret = err.getvalue() + out.getvalue()
            if len(ret) == 0:
                return None
            return ret

        except SystemExit:
            raise
        except:
            self.showtraceback()
            return self.resetoutput()

    def push(self, line: str) -> bool:
        if not hasattr(self, "output"):
            self.output = ""
        self.buffer.append(line)
        source = "\n".join(self.buffer)
        return self.runsource(source, self.filename)


debug_console = DebugConsole()
globals()
