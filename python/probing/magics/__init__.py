"""
A REPL (Read-Eval-Print Loop) implementation using an in-process IPython kernel.

This module provides a `CodeExecutor` class that encapsulates an IPython kernel
running within the same process. It allows for executing Python code, maintaining
state between executions, and defining custom "magic" commands.

The results of executions are encapsulated in an `ExecutionResult` object,
which can be easily serialized to JSON.
"""

from ipykernel.inprocess.manager import InProcessKernelManager
from jupyter_client.session import Session
from typing import Union, List, Optional
from dataclasses import dataclass, field, asdict
import json

from .torch_magic import TorchMagic
from .debug_magic import DebugMagic
from .stack_magic import StackMagic
from .handle_magic import HandleMagic


@dataclass
class ExecutionResult:
    """Encapsulates the result of a code execution.

    >>> res_ok = ExecutionResult(status='ok', output='hello')
    >>> print(res_ok.to_json(indent=2))
    {
      "status": "ok",
      "output": "hello",
      "traceback": []
    }
    >>> res_err = ExecutionResult(status='error', traceback=['line 1', 'line 2'])
    >>> print(res_err.to_json(indent=2))
    {
      "status": "error",
      "output": "",
      "traceback": [
        "line 1",
        "line 2"
      ]
    }
    """

    status: str  # 'ok' or 'error'
    output: str = ""
    traceback: Optional[List[str]] = field(default_factory=list)

    def to_json(self, indent: Optional[int] = None) -> str:
        """Serializes the result to a JSON string."""
        return json.dumps(asdict(self), indent=indent)

    def display(self):
        """Prints the execution result to the console."""
        print(f"Status: {self.status}")
        if self.output:
            print(f"Output:\n{self.output}")
        if self.traceback:
            print("Traceback:")
            for line in self.traceback:
                print(line)


class CodeExecutor:
    """A class that encapsulates an in-process IPython kernel for code execution.

    This class provides a simple interface to execute Python code in a persistent
    IPython kernel running within the same process. It handles the creation,
    communication, and shutdown of the kernel.

    By default, `InProcessKernelManager` uses a singleton `InteractiveShell`
    instance. This means that different `CodeExecutor` instances created in the
    same process will share the same underlying shell and, therefore, the same
    execution state (variables, imports, etc.).

    The executor also supports registering and using custom IPython magic commands.

    Attributes
    ----------
    km : InProcessKernelManager
        The kernel manager instance.
    kc : jupyter_client.inprocess.client.InProcessKernelClient
        The kernel client for communication.

    Examples
    --------
    >>> # Create two executor instances.
    >>> executor1 = CodeExecutor()
    >>> executor2 = CodeExecutor()
    >>> # They are different objects...
    >>> executor1 is executor2
    False
    >>> # ...but they share the same underlying kernel state.
    >>> _ = executor1.execute("my_var = 42")
    >>> res = executor2.execute("print(my_var)")
    >>> res.output
    '42'
    >>> # Clean up the resources.
    >>> executor1.shutdown() # doctest: +ELLIPSIS
    <BLANKLINE>
    Shutting down kernel...
    Kernel shut down.
    >>> executor2.shutdown() # doctest: +ELLIPSIS
    <BLANKLINE>
    Shutting down kernel...
    Kernel shut down.
    """

    def __init__(self):
        self.km = InProcessKernelManager()
        self.km.start_kernel()

        self.kc = self.km.client()
        self.kc.start_channels()

        if self.km.has_kernel:
            shell = self.km.kernel.shell
            shell.register_magics(TorchMagic(shell=shell))
            shell.register_magics(DebugMagic(shell=shell))
            shell.register_magics(StackMagic(shell=shell))
            shell.register_magics(HandleMagic(shell=shell))

    def execute(self, code_or_request: Union[str, dict]) -> ExecutionResult:
        """Executes a string of code or a request dictionary in the kernel.

        This method sends the code to the IPython kernel for execution and waits
        for the result. It captures stdout, stderr, and rich display outputs.

        The state of the kernel is preserved across calls. For example, variables
        or functions defined in one execution can be used in subsequent ones.

        Parameters
        ----------
        code_or_request : str or dict
            The code to execute as a string, or a dictionary conforming to the
            format `{'code': '...'}`.

        Returns
        -------
        ExecutionResult
            An object containing the status of the execution, the captured
            output, and any traceback if an error occurred.

        Examples
        --------
        >>> executor = CodeExecutor()
        >>> # Simple execution
        >>> res = executor.execute("a = 10; a + 5")
        >>> res.display()
        Status: ok
        Output:
        15
        >>> # Using a variable from a previous execution
        >>> res2 = executor.execute("print(f'The value of a is {a}')")
        >>> res2.display()
        Status: ok
        Output:
        The value of a is 10
        >>> # Handling an error
        >>> res3 = executor.execute("print(b)")
        >>> res3.display() # doctest: +ELLIPSIS
        Status: error
        Traceback:
        ...
        >>> executor.shutdown() # doctest: +ELLIPSIS
        <BLANKLINE>
        Shutting down kernel...
        Kernel shut down.
        """
        if isinstance(code_or_request, str):
            request = {"code": code_or_request}
        else:
            request = code_or_request

        # Execute the code, this is a non-blocking call
        self.kc.execute(request["code"], silent=False)

        # Wait for and get the execution result
        # For InProcessKernelClient, we can call get_shell_msg directly
        reply = self.kc.get_shell_msg(timeout=5)

        # Check execution status
        content = reply["content"]
        status = content["status"]

        if status == "error":
            traceback = content["traceback"]
            return ExecutionResult(status="error", traceback=traceback)

        # Get all stdout/stderr output from the IOPub channel
        output = []
        while self.kc.iopub_channel.msg_ready():
            sub_msg = self.kc.get_iopub_msg(timeout=5)
            msg_type = sub_msg["header"]["msg_type"]

            if msg_type == "stream":
                output.append(sub_msg["content"]["text"])
            elif msg_type == "execute_result":
                output.append(sub_msg["content"]["data"].get("text/plain", ""))

        result_text = "".join(output).strip()
        return ExecutionResult(status="ok", output=result_text)

    def shutdown(self):
        """Shuts down the kernel and its communication channels.

        This should be called to clean up resources when the executor is no
        longer needed. It stops the client channels and requests the kernel
        manager to shut down the kernel.
        """
        print("\nShutting down kernel...")
        self.kc.stop_channels()
        self.km.shutdown_kernel()
        print("Kernel shut down.")

import code

class DebugConsole(code.InteractiveConsole):
    def __init__(self):
        self.code_executor = CodeExecutor()
        super().__init__()
            
    def runsource(self, source):
        try:
            code = self.compile(source, "<input>", "single")
        except (OverflowError, SyntaxError, ValueError):
            print("Error in code:\n", source)
            retval = self.code_executor.execute(source)
            self.resetbuffer()
            return retval
        
        if code is None: #incomplete code
            return None
        
        retval = self.code_executor.execute(source)
        self.resetbuffer()
        return retval
    
    def push(self, code: str):
        """Pushes code to the executor and executes it.
        
        Examples
        --------
        >>> console = DebugConsole()
        >>> console.push("x = 10")
        '{"status": "ok", "output": "", "traceback": []}'
        >>> console.push("x")
        '{"status": "ok", "output": "10", "traceback": []}'
        >>> result = console.push("print(y)")
        >>> '"status": "error"' in result
        True
        >>> '"traceback":' in result
        True
        """
        try:
            self.buffer.append(code)
            source = "\n".join(self.buffer)
            retval = self.runsource(source)
            if retval is not None:
                return retval.to_json()
            return json.dumps({})
        except Exception as e:
            import traceback
            traceback.print_exc()