import time

import probing


tbl_0 = probing.ExternalTable("test000", ["allreduce_count", "broadcast_count"])
tbl_1 = probing.ExternalTable("test111", ["allreduce_count", "broadcast_count"], chunk_size=10000, discard_threshold=1000000000, discard_strategy="BaseMemorySize")
tbl_2 = probing.ExternalTable("test222", ["allreduce_count", "broadcast_count"], chunk_size=10, discard_threshold=10, discard_strategy="BaseMemorySize")
tbs = [tbl_0, tbl_1, tbl_2]


for tbl in tbs:
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
    