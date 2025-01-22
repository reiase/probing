use std::{collections::BTreeMap, sync::Arc, thread};

use anyhow::Result;
use inferno;

use crate::plugins::python::PythonPlugin;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Frame {
    stage: String,
    module: String,
}

pub fn query_profiling() -> Result<Vec<String>> {
    let data = thread::spawn(|| {
        let engine = probing_engine::create_engine()
            .with_plugin("probe", Arc::new(PythonPlugin::new("python")))
            .build()?;

        let query = r#"
        select module_name, stage, avg(duration) 
            from python.torch_profiling 
            group by module_name, stage
            order by (stage, module_name);
        "#;

        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { engine.query(query) })
    })
    .join()
    .map_err(|_| anyhow::anyhow!("error joining thread"))??;

    let mut frames = BTreeMap::default();

    for line in data.iter() {
        let frame = Frame {
            stage: line[1].to_string(),
            module: line[0].to_string(),
        };
        let duration = match line[2] {
            probing_proto::types::Value::Float32(x) => x as f64,
            probing_proto::types::Value::Float64(x) => x,
            _ => 0 as f64,
        };

        frames
            .entry(frame.clone())
            .and_modify(|x| *x += duration)
            .or_insert(duration);

        let mut parts = frame.module.split(".").collect::<Vec<_>>();
        if parts.len() > 1 {
            parts.pop();
            let parent = Frame {
                stage: frame.stage.clone(),
                module: parts.join("."),
            };
            frames.entry(parent).and_modify(|x| *x -= duration);
        }
    }

    Ok(frames
        .iter()
        .map(|(frame, duration)| {
            let mut line = String::default();
            line.push_str(&frame.stage);
            line.push(';');

            let parts = frame.module.split(".").collect::<Vec<_>>();
            for part in parts {
                line.push_str(part);
                line.push(';');
            }

            let duration = if *duration < 0. { 0. } else { *duration };

            line.push_str(&format!(" {}", (duration * 1000.) as isize));

            line
        })
        .collect())
}

pub fn flamegraph() -> String {
    let mut graph: Vec<u8> = vec![];
    let lines = query_profiling();
    if let Err(e) = lines {
        println!("Error: {}", e);
        return String::default();
    }
    if let Ok(lines) = lines {
        let lines = lines.iter().map(|x| x.as_str()).collect::<Vec<_>>();

        let mut opt = inferno::flamegraph::Options::default();
        opt.deterministic = true;
        // opt.factor = 0.001;
        match inferno::flamegraph::from_lines(&mut opt, lines, &mut graph) {
            Ok(_) => return String::from_utf8(graph).unwrap(),
            Err(e) => println!("Error: {}", e),
        }
    };
    String::from_utf8(graph).unwrap()
}
