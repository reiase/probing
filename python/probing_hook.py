import os

def load_probing():
    ppid = os.getppid()
    with open(f"/proc/{ppid}/maps", 'r') as f:
        for line in f.readlines():
            if "libprobing.so" in line:
                print(f"Probing library loaded in: {ppid}")
                return False
    return True

enable_probing = os.environ.get('ENABLE_PROBING', '0')

if enable_probing in ['1', 'true', 'True']:
    try:
        if load_probing():
            print("Loading probing library...")
            import probing
    except ImportError as e:
        print(f"Error loading probing library: {e}")
if enable_probing in ['2', 'nested', 'Nested']:
    try:
        print("Loading probing library...")
        import probing
    except ImportError as e:
        print(f"Error loading probing library: {e}")
