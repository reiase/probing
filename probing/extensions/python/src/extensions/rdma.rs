use probing_core::core::EngineCall;
use probing_core::core::EngineDatasource;
use probing_core::core::EngineError;
use probing_core::core::EngineExtension;
use probing_core::core::EngineExtensionOption;
use probing_core::core::Maybe;

use std::collections::HashMap;
use async_trait::async_trait;

use std::fs::File;
use std::io::{self, Read};
use std::time::{Duration, Instant};

#[derive(Debug, Default, EngineExtension)]
pub struct RdmaExtension {
    #[option(aliases=["sample.rate"])]
    sample_rate: Maybe<f64>,

    #[option(aliases=["hca.name"])]
    hca_name: Maybe<String>,
}

#[async_trait]
impl EngineCall for RdmaExtension {
    async fn call(
        &self,
        path: &str,
        params: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<Vec<u8>, EngineError> {
        println!("!!!RdmaExtension call with path: {}, params: {:?}, body: {:?}", path, params, body);
        if path == "" {
            println!("Handling RDMA request with params: {:?}", params);
            let mut monitor = RDMAMonitor::new("mlx5_cx6_0");

            const TEST_CALL: u32 = 5;
            let mut call_cnt = 0;

            while call_cnt < TEST_CALL {
                call_cnt += 1;
                monitor.obtain_newset();
                std::thread::sleep(Duration::from_millis(1000));
            }
            monitor.shutdown();
            return Ok("RDMA request handled successfully".as_bytes().to_vec());
        }
        Err(EngineError::UnsupportedCall)
    }
}


impl EngineDatasource for RdmaExtension {}

impl RdmaExtension {
    fn set_sample_rate(&mut self, sample_rate: Maybe<f64>) -> Result<(), EngineError> {
        if let Maybe::Just(rate) = sample_rate {
            if !(0.0..=1.0).contains(&rate) {
                return Err(EngineError::InvalidOptionValue(
                    Self::OPTION_SAMPLE_RATE.to_string(),
                    rate.to_string(),
                ));
            }
        }
        self.sample_rate = sample_rate;
        Ok(())
    }

    fn set_hca_name(&mut self, hca_name: Maybe<String>) -> Result<(), EngineError> {
        self.hca_name = hca_name;
        Ok(())
    }
}


struct RDMAMonitor {
    hca_name: String,
    previous_port_rcv_packets: Option<u64>,
    previous_port_xmit_packets: Option<u64>,
    last_measurement_time: Option<Instant>,
}

impl RDMAMonitor {
    fn new(hca_name: &str) -> Self {
        RDMAMonitor {
            hca_name: hca_name.to_string(),
            previous_port_rcv_packets: None,
            previous_port_xmit_packets: None,
            last_measurement_time: None,
        }
    }

    fn read_counter(&self, counter_name: &str) -> u64 {
        let path = if counter_name == "np_cnp_sent" || counter_name == "np_ecn_marked_roce_packets" {
            format!("/sys/class/infiniband/{}/ports/1/hw_counters/{}", self.hca_name, counter_name)
        } else {
            format!("/sys/class/infiniband/{}/ports/1/counters/{}", self.hca_name, counter_name)
        };

        match read_file_to_u64(&path) {
            Ok(value) => value,
            Err(e) => {
                println!("Error reading counter {}: {}", counter_name, e);
                0
            }
        }
    }

    fn calculate_rate(&self, current: Option<u64>, previous: Option<u64>, interval: Option<Duration>) -> f64 {
        if current.is_none() || previous.is_none() || interval.is_none() {
            return 0.0;
        }

        let current = current.unwrap();
        let previous = previous.unwrap();
        let interval = interval.unwrap().as_secs_f64();

        let diff = if current < previous {
            current.wrapping_add(2u64.pow(64)) - previous
        } else {
            current - previous
        };

        diff as f64 / interval
    }

    fn obtain_newset(&mut self) {
        let port_rcv_packets = self.read_counter("port_rcv_packets");
        let port_rcv_data = self.read_counter("port_rcv_data");
        let port_xmit_packets = self.read_counter("port_xmit_packets");
        let port_xmit_data = self.read_counter("port_xmit_data");
        let link_downed = self.read_counter("link_downed");
        let np_cnp_sent = self.read_counter("np_cnp_sent");
        let np_ecn_marked_roce_packets = self.read_counter("np_ecn_marked_roce_packets");

        let current_time = Instant::now();
        let interval = self.last_measurement_time.map(|t| current_time.duration_since(t));

        let rcv_pkts_rate = self.calculate_rate(
            Some(port_rcv_packets),
            self.previous_port_rcv_packets,
            interval,
        );

        let snd_pkts_rate = self.calculate_rate(
            Some(port_xmit_packets),
            self.previous_port_xmit_packets,
            interval,
        );

        self.previous_port_rcv_packets = Some(port_rcv_packets);
        self.previous_port_xmit_packets = Some(port_xmit_packets);
        self.last_measurement_time = Some(current_time);

        let new_data = [
            port_rcv_packets as f64,
            port_rcv_data as f64,
            port_xmit_packets as f64,
            port_xmit_data as f64,
            link_downed as f64,
            np_cnp_sent as f64,
            np_ecn_marked_roce_packets as f64,
            rcv_pkts_rate,
            snd_pkts_rate,
        ];

        println!("New RDMA data: {:?}", new_data);

        //TODO : send data to probe

        
    }

    fn shutdown(&self) {
        //TODO: shutdown
    }
}

fn read_file_to_u64(path: &str) -> io::Result<u64> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    contents.trim().parse::<u64>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}