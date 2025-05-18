def test_probing_python_value():
    import probing
    
    time_func = probing.query("select * from python.`time.time`")
    assert len(time_func) == 1
    
def test_probing_python_call():
    import probing
    
    time_func = probing.query("select * from python.`time.time()`")
    assert len(time_func) == 1
    
def test_probing_python_dict():
    import probing
    
    retval = probing.query("select * from python.`probing.inspect.get_dict()`")
    assert len(retval) == 1