import ctypes
dll = ctypes.CDLL("/home/yang/worksapce/probing/target/x86_64-unknown-linux-gnu/release/libprobing.so")

from dataclasses import dataclass

from enum import Enum, auto
from threading import Lock
from typing import Dict

import time


import probing

config_data = {
    "chunk_size": 10,
    "discard_threshold": 10,
    "discard_strategy": "BaseElementCount",
}

# config = probing.ExternalTableConfig(chunk_size=10, discard_threshold=10) #這個創建方式不行
tbl = probing.ExternalTable("test222", ["allreduce_count", "broadcast_count"], config_data)

tbl.append([1, 1])
tbl.append([2, 2])
tbl.append([3, 3])
tbl.append([1, 1])
tbl.append([2, 2])
tbl.append([3, 3])
tbl.append([1, 1])
tbl.append([2, 2])
tbl.append([3, 3])
tbl.append([1, 1])
tbl.append([2, 2])
tbl.append([3, 3])


while(True):
    time.sleep(8)
    try:
        input("Press enter to continue...")
    except KeyboardInterrupt:
        break
    