import time
import probing
import signal
import logging


logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger("RDMA_Monitor")

class GracefulKiller:
    """
    A class for handling graceful shutdown of the application when SIGINT or SIGTERM signals are received."""
    kill_now = False

    def __init__(self):
        signal.signal(signal.SIGINT, self.exit_gracefully)
        signal.signal(signal.SIGTERM, self.exit_gracefully)

    def exit_gracefully(self, signum, frame):
        self.kill_now = True
        logger.info("Ready to exit")


class RDMAMonitor():
    """
    A class to monitor RDMA statistics using an external table.
    
    This class extends probing.ExternalTable to provide a structured way to
    manage RDMA-related data.
    """
    def __init__(self, hca_name="mlx5_cx6_0", tbl_name="rdma_monitor_0"):
        self.table = probing.ExternalTable(
            tbl_name, 
            [
                "port_rcv_packets", 
                "port_rcv_data", 
                "port_xmit_packets", 
                "port_xmit_data",
                "link_downed",
                "np_cnp_sent",
                "np_ecn_marked_roce_packets",
                "rcv_pkts_rate",
                "snd_pkts_rate"
            ],
            chunk_size=10,
            discard_threshold=10,
            discard_strategy="BaseElementCount"
        )
        self.tbl_name = tbl_name
        self.hca_name = hca_name
        self._previous_port_rcv_packets = None
        self._previous_port_xmit_packets = None
        self._last_measurement_time = None
    
    def read_counter(self, counter_name):
        """Read hw counters from dir under /sys/class/infiniband"""
        try:
            if counter_name == "np_cnp_sent" or counter_name == "np_ecn_marked_roce_packets":
                with open(f"/sys/class/infiniband/{self.hca_name}/ports/1/hw_counters/{counter_name}", 'r') as f:
                    return int(f.read().strip())
            else:
                with open(f"/sys/class/infiniband/{self.hca_name}/ports/1/counters/{counter_name}", 'r') as f:
                    return int(f.read().strip())
        except Exception as e:
            print(f"Error reading counter {counter_name}: {e}")
            return 0

    def calculate_rate(self, current, previous, interval):
        if current is None or previous is None:
            return 0.0
        
        # Handle wrap-around for 64-bit counters
        if current < previous:
            current += 2**64 
        return (current - previous) / interval
    
    def obtain_newset(self):
        port_rcv_packets = self.read_counter("port_rcv_packets")
        port_rcv_data = self.read_counter("port_rcv_data")
        port_xmit_packets = self.read_counter("port_xmit_packets")
        port_xmit_data = self.read_counter("port_xmit_data")
        link_downed = self.read_counter("link_downed")
        np_cnp_sent = self.read_counter("np_cnp_sent")
        np_ecn_marked_roce_packets = self.read_counter("np_ecn_marked_roce_packets")

        current_time = time.time()
        interval = current_time - self._last_measurement_time if self._last_measurement_time else None
        
        rcv_pkts_rate = self.calculate_rate(
            port_rcv_packets, self._previous_port_rcv_packets, interval
        )
        
        snd_pkts_rate = self.calculate_rate(
            port_xmit_packets, self._previous_port_xmit_packets, interval
        )
        
        
        self._previous_port_rcv_packets = port_rcv_packets
        self._previous_port_xmit_packets = port_xmit_packets
        self._last_measurement_time = current_time

        new_data = [
            port_rcv_packets,
            port_rcv_data,
            port_xmit_packets,
            port_xmit_data,
            link_downed,
            np_cnp_sent,
            np_ecn_marked_roce_packets,
            rcv_pkts_rate,
            snd_pkts_rate
        ]
        
        logger.debug(f"New RDMA data: {new_data}")

        self.table.append(new_data) 
        
        return None

    def shutdown(self):
        self.table.drop(self.tbl_name)

    

if __name__ == "__main__":
    killer = GracefulKiller()  # Kill Signal

    try:
        monitor = RDMAMonitor(tbl_name="rdma_monitor_mlx0", hca_name="mlx5_cx6_0")

        logger.info("RDMAMonitor is Already running，Press Ctrl+C to exit...")
        while not killer.kill_now:
            monitor.obtain_newset()
            time.sleep(5)  # Sampling interval

    except Exception as e:
        logger.critical(f"RDMA Monitor Exception is: {str(e)}", exc_info=True)
    finally:
        if 'monitor' in locals():
            monitor.shutdown()
        logger.info("Already quit.")
        
