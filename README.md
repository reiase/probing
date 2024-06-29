### Probing: A Performance and Stability Diagnostic Tool for AI Applications

Probing is a performance and stability diagnostic tool designed specifically for AI applications. It aims to solve the debugging and optimization challenges of large-scale, distributed, long-duration heterogeneous computing tasks (such as LLM training and inference). By injecting a probing server into the target process, it can collect more detailed performance data or modify the execution behavior of the target process in real-time.

## Key Features

The main features of Probing include:

- **Debugging Capabilities**:
  - Observing the call stack, Python objects, Torch Tensors, and modules of the target process;
  - Supporting remote debugging through the DAP protocol using VSCode to debug the target process;
- **Performance Profiling**:
  - Sampling the performance of C/C++ code and generating flame graphs;
  - Supporting Torch profiling to analyze model performance;
- **Remote Control**:
  - Providing HTTP interfaces to retrieve data and control the execution of the target process;
  - Supporting remote injection of arbitrary Python code into the target process.

Compared to other debugging and diagnostic tools, `probing` is plug-and-play, allowing it to intrude into the target process at any time without interruption or restart, and without modifying the code.

## Quick Start

### Injecting the Probing

Use the following command to inject the probing:

```shell
probing <pid> inject [OPTIONS]
```

Options:
+ `-P, --pprof`: Enable profiling
+ `-c, --crash`: Enable crash handling
+ `-b, --background`: Enable background service
+ `-a, --address <ADDRESS>`: Specify the service listening address


### Diagnosing Issues

After injecting the probing, you can use the commands provided by probing to diagnose issues:

- `dump`: Print the current call stack to locate process blockages and deadlocks:

```shell
probing <pid> dump
```

- `pause`: Pause the process and start a remote debugging service:

```shell
probing <pid> pause [ADDRESS] # ADDRESS is optional, default is a random port
nc 127.0.0.1 3344           # Use nc to connect to the debugging service
```

- `catch`: Take over error handling and start a remote service upon error:

```shell
probing <pid> catch
```

- `listen`: Start the background debugging service:

```shell
probing <pid> listen [ADDRESS] # ADDRESS is optional, default is a random port
nc 127.0.0.1 3344            # Use nc to connect to the debugging service
```

- `execute`: Inject and execute code:

```shell
probing <pid> execute <SCRIPT>
# For example
probing <pid> execute script.py
probing <pid> execute "import traceback;traceback.print_stack()"
```

- `pprof`: Start profiling:

```shell
probing <pid> pprof

# Wait for a while and then get the flame graph
sleep 10
curl http://127.0.0.1:3344/flamegraph > flamegraph.svg

```

### Advanced Features

Probing also provides a series of Python analysis and diagnostic features for the development and debugging of large models:

- Activity Analysis: Capture the current Python stack information of each thread;
- Debugging: Start Python remote debugging to debug the target process in VSCode;
- Profiling: Profile the execution of torch models;
- Inspection: Inspect Python objects, torch Tensors, and torch Modules;

These features can be accessed through a web interface. For example, specify the service address when injecting the probing:

```shell
probing <pid> inject -b -a 127.0.0.1:1234
```

Then, you can access the above features by opening `http://127.0.0.1:1234` in a browser.

##Installing Probing

### Binary Installation

`probing` does not require special installation. Simply download the release file, extract it, and execute. Users can optionally add probing to the $PATH environment variable.

### Building from Source

`probing` relies on the trunk tool for building. Install it using the following command, or skip this step if it is already installed:

```shell
cargo install trunk
```

Once the build environment is ready, you can complete the build using the build.sh script:

```shell
sh build.sh
```

### Development Mode

To facilitate development, probing packages Python scripts and the web app into libprobing.so. Repacking every time code is modified can significantly reduce efficiency, so manual building is recommended:

```shell
# Continuously build the web app
cd app
trunk watch --filehash false -d dist/

# Build probing and libprobing
cargo b -p cli
cargo b
```

In debug mode, probing will automatically load the web app from the dist directory and Python scripts from src, eliminating the need for repacking.