import time

import probing

config_data = {
    "chunk_size": 10000,
    "discard_threshold": 1000000000,
    "discard_strategy": "BaseMemorySize",
}

# config_data = {
#     "chunk_size": 10,
#     "discard_threshold": 10,
#     "discard_strategy": "BaseElementCount",
# }

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
    