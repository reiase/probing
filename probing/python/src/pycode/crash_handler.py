def crash_handler(type, value, traceback):
    print(f"=============== Crash Handler ===============")
    print("type:", type)
    print("value:", value)
    print("traceback:", traceback)