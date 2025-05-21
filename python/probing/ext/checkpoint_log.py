import time
from dataclasses import dataclass
from probing.core.table import table

@table
@dataclass
class CheckpointLog:
    name: str
    elapsed: float
    start_time: float


def init():
    print("CheckpointLog init!", flush=True)
    _original_log = Timers.log
    def new_log(self, names, rank=None, normalizer=1.0, reset=True, barrier=False):

        result = _original_log(self, names, rank, normalizer, reset, barrier)
        
        for timer_name in ['save-checkpoint','save-checkpoint-non-persistent', 'load-checkpoint']:
            timer = self._timers.get(timer_name)
            if timer is not None:
                # 获取 elapsed（本次累计耗时，单位秒）
                elapsed = timer._elapsed
                # 获取 _start_time（最近一次 start 的时间戳，单位秒，time.time()）
                start_time = timer._start_time
                print(f"[MonkeyPatch] {timer_name}: elapsed={elapsed:.4f}s, _start_time={start_time}")
                CheckpointLog(
                    name=timer_name,
                    elapsed=elapsed,
                    start_time=start_time
                ).save()
        return result
    Timers.log = new_log

