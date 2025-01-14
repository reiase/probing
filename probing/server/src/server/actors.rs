use log::debug;

use actix::prelude::*;
use probing_proto::prelude::{Probe, ProbeCall};

pub struct ProbeActor {
    probe: Box<dyn Probe>,
}

impl ProbeActor {
    pub fn new(probe: Box<dyn Probe>) -> Self {
        Self { probe }
    }
}

impl Actor for ProbeActor {
    type Context = Context<Self>;
}

impl Handler<ProbeCall> for ProbeActor {
    type Result = ProbeCall;

    fn handle(&mut self, msg: ProbeCall, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("ProbeActor received message: {:?}", msg);
        match msg {
            ProbeCall::CallEnable(feature) => match self.probe.enable(&feature) {
                Ok(res) => ProbeCall::ReturnEnable(res),
                Err(err) => ProbeCall::Err(err.to_string()),
            },
            ProbeCall::CallDisable(feature) => match self.probe.disable(&feature) {
                Ok(res) => ProbeCall::ReturnDisable(res),
                Err(err) => ProbeCall::Err(err.to_string()),
            },
            ProbeCall::CallBacktrace(depth) => match self.probe.backtrace(depth) {
                Ok(res) => ProbeCall::ReturnBacktrace(res),
                Err(err) => ProbeCall::Err(err.to_string()),
            },
            ProbeCall::CallEval(code) => match self.probe.eval(&code) {
                Ok(res) => ProbeCall::ReturnEval(res),
                Err(err) => ProbeCall::Err(err.to_string()),
            },

            ProbeCall::CallFlamegraph => match self.probe.flamegraph() {
                Ok(res) => ProbeCall::ReturnFlamegraph(res),
                Err(err) => ProbeCall::Err(err.to_string()),
            },

            ProbeCall::Err(err) => ProbeCall::Err(err),
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod specs {
    use super::ProbeActor;
    use actix::Actor;
    use probing_proto::prelude::Probe;
    use probing_proto::prelude::ProbeCall;

    struct TestProbe;

    impl Probe for TestProbe {
        fn enable(&self, feture: &str) -> anyhow::Result<()> {
            match feture {
                "test" => Ok(()),
                _ => Err(anyhow::anyhow!("unknown feature")),
            }
        }

        fn disable(&self, feture: &str) -> anyhow::Result<()> {
            match feture {
                "test" => Ok(()),
                _ => Err(anyhow::anyhow!("unknown feature")),
            }
        }

        fn backtrace(
            &self,
            _tid: Option<i32>,
        ) -> anyhow::Result<Vec<probing_proto::protocol::process::CallFrame>> {
            Err(anyhow::anyhow!("not implemented"))
        }

        fn eval(&self, code: &str) -> anyhow::Result<String> {
            match code {
                "test" => Ok("test".to_string()),
                _ => Err(anyhow::anyhow!("unknown code")),
            }
        }
    }

    #[actix::test]
    async fn test_probe_actor() {
        let probe = ProbeActor::new(Box::new(TestProbe)).start();

        assert_eq!(
            probe
                .send(ProbeCall::CallEnable("test".to_string()))
                .await
                .unwrap(),
            ProbeCall::ReturnEnable(())
        );

        assert_eq!(
            probe
                .send(ProbeCall::CallEval("test".to_string()))
                .await
                .unwrap(),
            ProbeCall::ReturnEval("test".to_string())
        )
    }
}
