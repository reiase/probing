import os
import uuid
import requests
from datetime import datetime



# API基础URL - 需要根据实际情况修改
API_BASE_URL = "http://your-api-endpoint.com"

def start_job():
    """
    记录作业开始信息
    """
    try:
        job_id = os.getenv('JOB_ID')
        if not job_id:
            return

        start_time = datetime.now().isoformat()
        job_uuid = str(uuid.uuid4())

        data = {
            "job_id": job_id,
            "start_time": start_time,
            "uuid": job_uuid
        }

        response = requests.post(f"{API_BASE_URL}/job/start", json=data)
        response.raise_for_status()
        return job_uuid

    except Exception as e:
        raise

def end_job():
    """
    记录作业结束信息
    """
    try:
        job_id = os.getenv('JOB_ID')
        if not job_id:
            return

        end_time = datetime.now().isoformat()

        data = {
            "job_id": job_id,
            "end_time": end_time
        }

        response = requests.post(f"{API_BASE_URL}/job/end", json=data)
        response.raise_for_status()

    except Exception as e:
        raise

def record_error(error_message):
    """
    记录作业错误信息
    """
    try:
        job_id = os.getenv('JOB_ID')
        if not job_id:
            return

        error_time = datetime.now().isoformat()

        data = {
            "job_id": job_id,
            "error_time": error_time,
            "error_message": str(error_message)
        }

        response = requests.post(f"{API_BASE_URL}/job/error", json=data)
        response.raise_for_status()

    except Exception as e:
        raise

# 使用示例
if __name__ == "__main__":
    try:
        # 开始作业
        job_uuid = start_job()
        
        # 这里放置你的主要作业代码
        # ...
        
        # 结束作业
        end_job()
        
    except Exception as e:
        # 记录错误
        record_error(e)
        raise 