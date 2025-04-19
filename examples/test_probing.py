#!/usr/bin/env python3
"""
Hierarchical test script for probing_hook.

Tests probing behavior across parent and child processes,
displaying results in a tree structure.

Example:
    PROBE=0 python test_probing.py
    PROBE=1 python test_probing.py
    PROBE=2 python test_probing.py --depth 3
    PROBE=nested python test_probing.py --children 2 --depth 2
    PROBE=regex:test_.* python test_probing.py --depth 2
"""
import os
import sys
import subprocess
import argparse
import io
import contextlib

def get_process_summary():
    """Get a one-line summary of the current process status"""
    # Capture stderr during import to prevent it from messing up our output
    stderr_output = io.StringIO()
    with contextlib.redirect_stderr(stderr_output):
        # Import probing_hook which will conditionally import probing
        import probing_hook
    
    # Check if probing module is loaded
    probing_imported = 'probing' in sys.modules
    pid = os.getpid()
    script_name = os.path.basename(sys.argv[0])
    probe_value = os.environ.get('PROBE', 'Not set')
    
    # Get any stderr output from the import
    stderr_msg = stderr_output.getvalue().strip()
    
    summary = f"PID:{pid} Script:{script_name} PROBE:{probe_value} Loaded:{probing_imported}"
    if stderr_msg:
        # Add import message as part of the summary, cleaned up
        import_msg = stderr_msg.replace('\n', ' ')
        summary += f" ({import_msg})"
    
    return summary

def main():
    parser = argparse.ArgumentParser(description='Test probing hook with child processes')
    parser.add_argument('--depth', type=int, default=1, help='Depth of process tree')
    parser.add_argument('--children', type=int, default=2, help='Number of children per process')
    parser.add_argument('--current-depth', type=int, default=0, help=argparse.SUPPRESS)
    parser.add_argument('--prefix', type=str, default='', help=argparse.SUPPRESS)
    args = parser.parse_args()
    
    # Get the current process summary
    summary = get_process_summary()
    
    lines = []
    # Print with appropriate indentation and tree structure
    if args.current_depth == 0:
        print(f"└─ {summary}")
    else:
        print(f"{args.prefix}└─ {summary}")
    
    # If we haven't reached max depth, spawn child processes
    if args.current_depth < args.depth:
        for i in range(args.children):
            # Last child uses a different prefix to maintain proper tree structure
            if i == args.children - 1:
                next_prefix = args.prefix + "   "
            else:
                next_prefix = args.prefix + "│  "
                
            # Build command for the child process
            cmd = [
                sys.executable, "examples/child_test_probing.py",
                '--current-depth', str(args.current_depth + 1),
                '--depth', str(args.depth),
                '--children', str(args.children),
                '--prefix', next_prefix
            ]
            
            # Run child process and capture its output
            proc = subprocess.run(
                cmd, 
                env=os.environ.copy(),
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,  # Discard stderr from child processes
                text=True
            )
                        
            # Print child process output
            if proc.stdout:
                print(proc.stdout, end='')

if __name__ == "__main__":
    main()