from typing import Any
from IPython.core.magic import Magics, magics_class, line_magic
import urllib.parse
import gc
import json
import torch

class _obj_:
    def __init__(self, obj):
        self._obj = obj

    def __repr__(self):
        return json.dumps(self._obj, indent=2)

@magics_class
class HandleMagic(Magics):

    def parse_query(self, query: str) -> Any:
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

    @line_magic
    def get_objects(self, line: str):
        """Get objects from memory."""
        args = dict(item.split("=") for item in line.split()) if line else {}
        type_selector = args.get("type", None)
        limit = args.get("limit", None)
        limit = int(limit) if limit is not None else None

        class _obj_list_:
            def __init__(self, objs):
                self._objs = objs

            def __repr__(self):
                return json.dumps(self._objs, indent=2)

        objs = gc.get_objects()
        objs = [obj for obj in objs if self._filter_obj_type(obj, type_selector)]
        objs = objs[:limit] if limit is not None else objs
        return _obj_list_([self._get_obj_repr(obj) for obj in objs])

    @line_magic
    def get_torch_tensors(self, line: str):
        """Get torch tensors from memory."""
        args = dict(item.split("=") for item in line.split()) if line else {}
        limit = args.get("limit", None)
        objs = gc.get_objects()
        objs = [obj for obj in objs if self._filter_obj_type(obj, "torch.Tensor")]
        objs = objs[: int(limit)] if limit is not None else objs
        return _obj_([self._get_obj_repr(obj) for obj in objs])

    @line_magic
    def get_torch_modules(self, line: str):
        """Get torch modules from memory."""
        args = dict(item.split("=") for item in line.split()) if line else {}
        limit = args.get("limit", None)
        toplevel = args.get("toplevel", "False").lower() in ("true", "1", "t")

        limit = int(limit) if limit is not None else None

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
