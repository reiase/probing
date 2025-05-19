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
    
    df = probing.query("select * from python.`probing.inspect.get_dict()`")
    assert (df.columns == ['int', 'float', 'str']).all()
    assert len(df) == 1
    
def test_probing_python_list():
    import probing
    
    df = probing.query("select * from python.`probing.inspect.get_list()`")
    assert (df.columns == ['value']).all()
    assert len(df) == 3
    
def test_probing_python_dict_list():
    import probing
    
    df = probing.query("select * from python.`probing.inspect.get_dict_list()`")
    assert (df.columns == ['int', 'float', 'str']).all()
    assert len(df) == 2
    assert df["int"][0] == '1'
    assert df["int"][1] == '2'
    assert df["float"][0] == '1.0'
    assert df["float"][1] == '2.0'
    assert df["str"][0] == 'str'
    assert df["str"][1] == 'str2'
