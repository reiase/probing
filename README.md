# Probing: Performance and Stability Diagnostic Tool for AI Applications

Probing is a performance and stability diagnostic tool designed specifically for AI applications. It aims to solve debugging and optimization challenges in large-scale, distributed, and long-cycle AI heterogeneous computing tasks (such as LLM training and inference). By injecting a probing server into the target process, it can collect more detailed performance data or modify the execution behavior of the target process in real-time.

## Main Features

The main features of Probing include:

- **Debugging Functions:**
  - Observe the call stack, Python objects, Torch Tensors, and modules of the target process;
  - Support remote debugging, allowing the use of VSCode for remote debugging of the target process via the DAP protocol;

- **Performance Analysis:**
  - Perform performance sampling on C/C++ code and generate flame graphs;
  - Support profiling functions for Torch, analyzing model performance;

- **Remote Control:**
  - Provide HTTP interfaces for data retrieval and control of the target process execution;
  - Support remote injection of any Python code into the target process.

Compared to other debugging and diagnostic tools, `probing` can be used immediately without the need to interrupt or restart the target process, nor does it require code modifications.

## Quick Start

### Probing Injection

**Inject via Command Line**

```shell
probing --pid <pid> inject [OPTIONS]
```

Options: `-P,--pprof` to enable profiling; `-c,--crash` to enable crash handling; `-l,--listen <ADDRESS>` to specify the address for the service to listen for remote connections.

**Inject via Code**

```python
import probing
probing.init(listen="127.0.0.1:9922")
```

### Debugging and Performance Diagnosis

After injecting probing, you can use the commands provided by probing for issue diagnosis:

- `debug` command (aliases: `dbg` or `d`): Debugging and inspection tool for locating process blockages and deadlock issues;

    ```sh
    $ probing help debug
    Debug and Inspection Tool

    Usage: probing debug [OPTIONS]

    Options:
      -d, --dump               Dump the calling stack of the target process
      -p, --pause              Pause the target process and listen for remote connection
      -a, --address <ADDRESS>  address to listen [default: 127.0.0.1:9922]
      -h, --help               Print help
    ```

    Examples:

    ```sh
    $ probing -p <pid> debug --dump # Print the current call stack of the target process
    $ probing -p <pid> d -d         # Same as above, using the short command

    $ probing -p <pid> debug --pause --address 127.0.0.1:9922 # Pause the target process and wait for remote connection
    $ probing -p <pid> d -p -a 127.0.0.1:9922                 # Same as above, using the short command
    ```

- `performance` command (aliases: `perf` or `p`): Performance diagnosis tool for collecting performance data and diagnosing performance bottlenecks;

    ```sh
    $ probing help performance
    Performance Diagnosis Tool

    Usage: probing performance [OPTIONS]

    Options:
          --cc     profiling c/c++ codes
          --torch  profiling torch models
      -h, --help   Print help
    ```

    Examples:

    ```sh
    $ probing -p <pid> perf --cc    # Enable profiling for C/C++ code, can output flamegraph
    $ probing -p <pid> perf --torch # Enable profiling for Torch
    ```

### Advanced Features

Probing provides a series of Python analysis and diagnostic functions for large model development and debugging:

- **Activity Analysis:** Capture the current Python stack information for each thread;
- **Debug Function:** Enable Python remote debugging, allowing debugging of the target process in VSCode;
- **Profile Function:** Perform profiling on Torch model execution;
- **Inspect Function:** Inspect Python objects, Torch Tensor objects, and Torch Module models;

These functions can be accessed via the web interface. Specify the service address when injecting probing, for example:

```shell
probing <pid> inject -b -a 127.0.0.1:1234
```

Then you can use the above functions by opening `http://127.0.0.1:1234` in a browser.

## Installing Probing

### Binary Installation

`probing` can be installed via pip:

```sh
$ pip install probing
```

### Building from Source

`probing` relies on the `trunk` tool for building, which can be installed with the following command. If already installed, you can skip this step:

```shell
cargo install trunk
```

After preparing the build environment, you can build the python package with:

```shell
make
```

### Development Mode

To facilitate user usage, probing packages the Python scripts and web app into `libprobing.so`. Rebuilding the package every time the code is modified can greatly reduce efficiency. Therefore, manual build is recommended:

```shell
# Continuously build the web app
cd app
trunk watch --filehash false -d dist/

# Build probing and libprobing
cargo b -p cli
cargo b
```

In debug mode, `probing` will automatically load the web app from the `dist` directory and the Python scripts from `src/`, without the need for repackaging.

## License

This project is licensed under the GNU General Public License v3.0. See the [LICENSE](./LICENSE) file for more details.
