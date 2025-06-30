import sys
import threading

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
        if value and "cpu" in ret["device"]:
            ret["value"] = str(obj)[:150]
    else:
        if value:
            ret["value"] = str(obj)[:150]
    return ret


if "tid" not in locals():
    tid = threading.get_native_id()

def get_threads():
    for thread in threading.enumerate():
        if thread.native_id == tid:
            return thread.ident

stacks = []
frames = sys._current_frames()
tid, nid = get_threads(), tid

if tid in frames:
    curr = frames[tid]
    while curr is not None:
        stack = {"PyFrame": {
            "file": curr.f_code.co_filename,
            "func": curr.f_code.co_name,
            "lineno": curr.f_lineno,
            "locals": {
                k: _get_obj_repr(v, value=True) for k, v in curr.f_locals.items()
            },
        }}
        stacks.append(stack)
        curr = curr.f_back
    import json
    retval = json.dumps(stacks)
else:
    retval = None