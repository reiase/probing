def test_enable_disable_python_ext():
    import probing

    probing.query("set probing.python.enabled=`probing.ext.example`")

    table_names = probing.query("show tables")["table_name"].to_list()
    assert "example_ext" in table_names

    probing.query("set probing.python.disabled=`probing.ext.example`")
    table_names = probing.query("show tables")["table_name"].to_list()
    assert "example_ext" not in table_names


def test_reenable_python_ext():
    import probing

    probing.query("set probing.python.enabled=`probing.ext.example`")

    table_names = probing.query("show tables")["table_name"].to_list()
    assert "example_ext" in table_names

    probing.query("set probing.python.disabled=`probing.ext.example`")
    table_names = probing.query("show tables")["table_name"].to_list()
    assert "example_ext" not in table_names

    probing.query("set probing.python.enabled=`probing.ext.example`")
    table_names = probing.query("show tables")["table_name"].to_list()
    assert "example_ext" in table_names
