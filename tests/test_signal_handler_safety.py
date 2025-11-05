#!/usr/bin/env python3
"""
Test script to verify signal-based stack collection functionality.

This test validates that the signal handler can successfully collect
stack traces without crashing or deadlocking.
"""
import os
import sys
import time
import signal
import subprocess
import tempfile

def busy_work():
    """Perform some CPU work to have interesting stack frames"""
    result = 0
    for i in range(1000000):
        result += i * i
    return result

def test_signal_handler():
    """Test that the process can be interrupted and queried for backtraces"""
    # Start a simple Python script with probing enabled
    test_script = """
import time
import sys

def recursive_function(depth):
    if depth == 0:
        # Busy loop to ensure we can be interrupted
        for i in range(10):
            sum([x*x for x in range(1000)])
            time.sleep(0.1)
        return
    return recursive_function(depth - 1)

if __name__ == "__main__":
    print("READY", flush=True)  # Signal we're ready to be profiled
    recursive_function(5)
    print("DONE", flush=True)
"""
    
    # Create temporary file for test script
    with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
        script_path = f.name
        f.write(test_script)
    
    try:
        # Start process with probing enabled
        env = os.environ.copy()
        env['PROBING'] = '1'
        
        proc = subprocess.Popen(
            [sys.executable, script_path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=env
        )
        
        try:
            # Wait for ready signal
            line = proc.stdout.readline()
            if "READY" not in line:
                print(f"ERROR: Expected READY signal, got: {line}")
                return False
            
            pid = proc.pid
            print(f"✓ Test process started (PID: {pid})")
            
            # Try to collect backtrace using probing CLI
            # This will trigger the signal handler
            result = subprocess.run(
                ['probing', '-t', str(pid), 'backtrace'],
                capture_output=True,
                text=True,
                timeout=5
            )
            
            if result.returncode == 0:
                print("✓ Backtrace collection succeeded")
                print(f"  Output preview: {result.stdout[:200]}...")
                success = True
            else:
                print(f"✗ Backtrace collection failed: {result.stderr}")
                success = False
            
            # Wait for process to complete
            try:
                proc.wait(timeout=5)
                print("✓ Test process completed normally")
            except subprocess.TimeoutExpired:
                print("✗ Test process timed out (possible deadlock)")
                proc.kill()
                success = False
            
            return success
            
        except Exception as e:
            print(f"✗ Test failed with exception: {e}")
            return False
        finally:
            # Cleanup subprocess
            if proc.poll() is None:
                proc.kill()
                try:
                    proc.wait(timeout=1)
                except subprocess.TimeoutExpired:
                    pass
    finally:
        # Cleanup temp file
        try:
            os.remove(script_path)
        except (OSError, FileNotFoundError):
            pass

def test_repeated_signals():
    """Test that repeated signal delivery doesn't cause issues"""
    print("\n=== Testing Repeated Signal Delivery ===")
    
    test_script = """
import time
for i in range(20):
    sum([x*x for x in range(10000)])
    time.sleep(0.05)
"""
    
    # Create temporary file for test script
    with tempfile.NamedTemporaryFile(mode='w', suffix='.py', delete=False) as f:
        script_path = f.name
        f.write(test_script)
    
    try:
        env = os.environ.copy()
        env['PROBING'] = '1'
        
        proc = subprocess.Popen(
            [sys.executable, script_path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            env=env
        )
    
    try:
        time.sleep(0.1)  # Let process start
        pid = proc.pid
        
        # Send multiple backtrace requests
        success_count = 0
        for i in range(5):
            result = subprocess.run(
                ['probing', '-t', str(pid), 'backtrace'],
                capture_output=True,
                text=True,
                timeout=3
            )
            if result.returncode == 0:
                success_count += 1
            time.sleep(0.2)
        
        print(f"✓ Successfully collected {success_count}/5 backtraces")
        
        # Ensure process completes
        proc.wait(timeout=5)
        print("✓ Process completed without hanging")
        
        return success_count >= 3  # At least 3/5 should succeed
        
    except Exception as e:
        print(f"✗ Repeated signals test failed: {e}")
        proc.kill()
        return False
    finally:
        if proc.poll() is None:
            proc.kill()
            try:
                proc.wait(timeout=1)
            except subprocess.TimeoutExpired:
                pass
        try:
            os.remove(script_path)
        except (OSError, FileNotFoundError):
            pass

def main():
    print("=== Signal Handler Safety Test ===\n")
    print("This test verifies that the signal-based stack collection")
    print("works correctly and doesn't cause deadlocks or crashes.\n")
    
    # Check if probing CLI is available
    try:
        subprocess.run(['probing', '--version'], capture_output=True, timeout=2)
    except (subprocess.SubprocessError, FileNotFoundError):
        print("⚠ Probing CLI not found. Please build and install probing first.")
        print("  Run: cargo build -p probing-cli")
        return 1
    
    print("=== Test 1: Basic Signal Handler ===")
    test1 = test_signal_handler()
    
    test2 = test_repeated_signals()
    
    print("\n=== Test Summary ===")
    if test1 and test2:
        print("✓ All tests passed!")
        print("\nThe signal handler successfully collected stack traces without")
        print("deadlocking or crashing, despite using non-async-signal-safe code.")
        print("\nSee docs/signal-handler-safety.md for detailed explanation of")
        print("the trade-offs and safer alternatives.")
        return 0
    else:
        print("✗ Some tests failed")
        if not test1:
            print("  - Basic signal handler test failed")
        if not test2:
            print("  - Repeated signals test failed")
        return 1

if __name__ == "__main__":
    sys.exit(main())
