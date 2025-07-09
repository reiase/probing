import threading
import time
import os
from typing import Callable, Dict, Any
from external_table_rdma import RDMAMonitor

class DataCollector:
    ENABLED_ENV_VAR = "RDMA_COLLECTOR_ENABLED"
    INTERVAL_ENV_VAR = "RDMA_COLLECTOR_INTERVAL"
    
    def __init__(self, default_config: Dict[str, Any] = None):
        self._lock = threading.Lock()
        self._stop_event = threading.Event()
        self._thread = None
        self._collector_fn = self._default_collector
        
        self._config = self._load_config_from_env(default_config or {})
        
        if self._config.get("enabled", False):
            self._start_collector()
            
    def _load_config_from_env(self, default_config: Dict[str, Any]) -> Dict[str, Any]:
        config = default_config.copy()
        
        enabled_str = os.getenv(self.ENABLED_ENV_VAR, str(config.get("enabled", "false"))).lower()
        config["enabled"] = enabled_str in {"true", "1", "yes"}
        
        interval_str = os.getenv(self.INTERVAL_ENV_VAR, str(config.get("interval", "5.0")))
        try:
            config["interval"] = float(interval_str)
        except ValueError:
            print(f"Invalid interval value {interval_str} using default interval 5.0")
            config["interval"] = 5.0
            
        return config
    
    def refresh_config_from_env(self) -> None:
        with self._lock:
            old_config = self._config.copy()
            self._config = self._load_config_from_env(old_config)
            
            if self._config["enabled"] and not old_config["enabled"]:
                self._start_collector()
            elif not self._config["enabled"] and old_config["enabled"]:
                self._stop_collector()
    
    def get_config(self) -> Dict[str, Any]:
        with self._lock:
            return self._config.copy()
            
    def register_collector(self, collector_fn: Callable) -> None:
        self._collector_fn = collector_fn
    
    def _default_collector(self) -> None:
        print("Collecting .......:", time.time())
    
    def _collector_loop(self) -> None:
        while not self._stop_event.is_set():
            interval = self._config.get("interval", 1.0)
            
            try:
                self._collector_fn()
            except Exception as e:
                print(f"Collector error: {e}")
                
            self._stop_event.wait(interval)
    
    def _start_collector(self) -> None:
        if self._thread is None or not self._thread.is_alive():
            self._stop_event.clear()
            self._thread = threading.Thread(target=self._collector_loop, daemon=True)
            self._thread.start()
            print("Collector thread started.")
    
    def _stop_collector(self) -> None:
        if self._thread is not None and self._thread.is_alive():
            self._stop_event.set()
            self._thread.join(timeout=2.0)
            print("Collector thread stopped.")

    def shutdown(self) -> None:
        with self._lock:
            self._config["enabled"] = False
            self._stop_collector()


if __name__ == "__main__":
    os.environ["RDMA_COLLECTOR_ENABLED"] = "true"
    os.environ["RDMA_COLLECTOR_INTERVAL"] = "3.0"
    
    collector = DataCollector()
    
    monitor = RDMAMonitor(tbl_name="rdma_monitor_mlx0", hca_name="mlx5_cx6_0")

    collector.register_collector(monitor.obtain_newset)
    
    time.sleep(15)
    
    os.environ["DATA_COLLECTOR_ENABLED"] = "false"
    collector.refresh_config_from_env()
    
    collector.shutdown()
