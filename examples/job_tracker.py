import os
import uuid
import requests
from datetime import datetime



# API基础URL - 需要根据实际情况修改
API_BASE_URL = "http://logi-core.hecp:32245/api"
job_uuid = str(uuid.uuid4())

def start_job():
    """
    记录作业开始信息
    """
    try:
        job_id = os.getenv('JOB_ID')
        if not job_id:
            return
        world_size = os.getenv('WORLD_SIZE')
        if not world_size:
            return
        pod_ip = os.getenv('POD_IP')
        if not pod_ip:
            return
        timestamp = datetime.now().isoformat()
        

        data = {
            "jobId": job_id,
            "timestamp": timestamp,
            "pod_ip": pod_ip,
            "worldSize": world_size,
            "uuid": job_uuid
        }

        response = requests.post(f"{API_BASE_URL}/job/start", json=data)
        response.raise_for_status()

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

        timestamp = datetime.now().isoformat()

        data = {
            "jobId": job_id,
            "timestamp": timestamp,
            "uuid": job_uuid
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

        timestamp = datetime.now().isoformat()

        data = {
            "jobId": job_id,
            "timestamp": timestamp,
            "errorMessage": str(error_message),
            "uuid": job_uuid
        }

        response = requests.post(f"{API_BASE_URL}/job/error", json=data)
        response.raise_for_status()

    except Exception as e:
        raise