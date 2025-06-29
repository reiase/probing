# nogil_sleep.pyx
# Cython实现的不释放GIL的sleep函数

from libc.time cimport nanosleep, timespec
from libc.errno cimport errno, EINTR
cimport cython

@cython.cdivision(True)
def sleep_nogil(double seconds):
    """
    不释放GIL的睡眠函数
    
    Args:
        seconds: 睡眠时间（秒），支持小数
    """
    cdef timespec req, rem
    cdef int result
    
    # 将秒数转换为timespec结构
    req.tv_sec = <long>seconds
    req.tv_nsec = <long>((seconds - <double>req.tv_sec) * 1000000000)
    
    # 循环调用nanosleep，处理被信号中断的情况
    while True:
        # 调用nanosleep，不释放GIL
        with nogil:
            result = nanosleep(&req, &rem)
        
        if result == 0:
            # 睡眠成功完成
            break
        elif errno == EINTR:
            # 被信号中断，继续睡眠剩余时间
            req = rem
            continue
        else:
            # 其他错误
            raise OSError(f"nanosleep failed with errno {errno}")

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

@cython.cdivision(True) 
def precise_sleep_nogil(double seconds):
    """
    精确的不释放GIL睡眠函数，结合了nanosleep和忙等待
    
    Args:
        seconds: 睡眠时间（秒），支持小数
    """
    cdef double threshold = 0.001  # 1毫秒阈值
    
    if seconds > threshold:
        # 对于较长的睡眠时间，先用nanosleep
        sleep_nogil(seconds - threshold)
        # 最后用忙等待确保精确性
        busy_sleep_nogil(threshold)
    else:
        # 对于短时间，直接用忙等待
        busy_sleep_nogil(seconds)
