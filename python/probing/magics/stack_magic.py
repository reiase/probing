from IPython.core.magic import Magics, magics_class, line_magic
import traceback
import sys
import json

def _get_obj_type(obj):
    try:
        m = type(obj).__module__
        n = type(obj).__name__
        return f"{m}.{n}"
    except Exception:
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

class _obj_:
    def __init__(self, obj):
        self._obj = obj

    def __repr__(self):
        return json.dumps(self._obj, indent=2)

@magics_class
class StackMagic(Magics):

    @line_magic
    def bt(self, line: str):
        """Print python and C stack."""
        py = "".join(traceback.format_stack())
        return f"{py}"

    @line_magic
    def dump_stack(self, line: str):
        """Dump stack frames."""
        stacks = []

        curr = sys._getframe(1)
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
