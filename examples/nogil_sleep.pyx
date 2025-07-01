cimport cython


def busy_sleep_nogil(double seconds):
    """
    忙等待的睡眠函数（不释放GIL，占用CPU）
    
    Args:
        seconds: 睡眠时间（秒），支持小数
    """
    from time import time
    cdef double start_time = time()
    cdef double current_time
    cdef double target_time = start_time + seconds
    
    # 忙等待循环，不释放GIL
    while True:
        current_time = time()
        if current_time >= target_time:
            break
        # 短暂的CPU让步，但不释放GIL
        pass
