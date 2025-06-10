import os
from datetime import datetime
import atexit
import sys
import requests
import probing


# API基础URL - 需要根据实际情况修改
API_BASE_URL = "http://logi-core.hecp:32245/api"
JOB_UNIQUE_ID = datetime.now().strftime("%Y%m%d%H%M%S%f") # Timestamp-based ID
JOB_ID = os.getenv('JOB_ID') or os.getenv('JOB_NAME', 'unknown_job')

def start_job_hook():
    if int(os.getenv('GROUP_WORLD_SIZE')) == int(os.getenv('GROUP_RANK'))+1 and int(os.getenv('LOCAL_RANK')) == 0:
        """
        记录作业开始信息
        """
        print("Job tracker: start_job_hook called.")
        world_size = os.getenv('WORLD_SIZE', 'N/A')
        tq_gpu_num = os.getenv('TQ_GPU_NUM', 'N/A')
        print(f"Job started: ID={JOB_ID}, TimestampID={JOB_UNIQUE_ID}, WorldSize={world_size}, TQ_GPU_NUM={tq_gpu_num}")
        data = {
                "jobId": JOB_ID,
                "timestamp": datetime.now().timestamp() * 1_000_000,
                "worldSize": world_size,
                "tqGpuNum": tq_gpu_num,
                "uuid": JOB_UNIQUE_ID
            }
        response = requests.post(f"{API_BASE_URL}/job/start", json=data)
        response.raise_for_status()

def end_job_hook():
    if int(os.getenv('GROUP_WORLD_SIZE')) == int(os.getenv('GROUP_RANK'))+1 and int(os.getenv('LOCAL_RANK')) == 0:
        """
        记录作业结束信息 (通过 atexit 注册)
        """
        print("Job tracker: end_job_hook called via atexit.")
        print(f"Job ended: ID={JOB_ID}, TimestampID={JOB_UNIQUE_ID}")
        timestamp = datetime.now().timestamp() * 1_000_000

        s=" where timestamp > " + str(timestamp - 30000000)
        df0 = probing.query("select * from python.iter_output_trace" + s)
        df1 = probing.query("select * from python.checkpoint_log" + s)
        data = {
                "jobId": JOB_ID,
                "timestamp": timestamp,
                "uuid": JOB_UNIQUE_ID,
                "iter": df0.to_dict(orient='records'),
                "checkpoint": df1.to_dict(orient='records')
        }
        response = requests.post(f"{API_BASE_URL}/job/end", json=data)
        response.raise_for_status()

def record_error_hook(exc_type, exc_value, exc_traceback):
    """
    记录作业错误信息 (通过 sys.excepthook 注册)
    """
    print(f"Job tracker: record_error_hook called for exception: {exc_value}")
    # 调用原始的 excepthook，以便错误仍然被打印到 stderr
    sys.__excepthook__(exc_type, exc_value, exc_traceback)
    print(f"Job error: ID={JOB_ID}, TimestampID={JOB_UNIQUE_ID}, Error={str(exc_value)}")
    timestamp = datetime.now().timestamp() * 1_000_000
    data = {
        "jobId": JOB_ID,
        "timestamp": timestamp,
        "errorMessage": str(exc_value),
        "uuid": JOB_UNIQUE_ID
    }

    response = requests.post(f"{API_BASE_URL}/job/error", json=data)
    response.raise_for_status()

# 注册钩子
print("Job tracker: Registering hooks...")
atexit.register(end_job_hook)
sys.excepthook = record_error_hook

# 作业开始时立即调用
start_job_hook()

print(f"Job tracker: Script initialized with JOB_ID: {JOB_ID}, TimestampID: {JOB_UNIQUE_ID}")
print("Job tracker: Waiting for program to exit or error to occur...")

# 为了演示，可以放一个简单的应用逻辑，或者让脚本等待
# import time
# print("Main program logic running...")
# time.sleep(5) # 模拟作业运行
# raise ValueError("This is a test error to trigger excepthook") # 取消注释以测试错误钩子
# print("Main program logic finished.")
