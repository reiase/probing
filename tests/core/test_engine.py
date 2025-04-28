from json import load


def test_query():
    import pandas as pd

    from probing import query

    sql = "SELECT 1 AS a, 2 AS b"
    df = query(sql)
    assert isinstance(df, pd.DataFrame)
    assert df.shape == (1, 2)
    assert df.columns.tolist() == ["a", "b"]
    assert df["a"].tolist() == [1]
    assert df["b"].tolist() == [2]
    
def test_load_extension():
    import sys
    from probing import load_extension
    
    statement = "probing.ext.example"
    load_extension(statement)
    
    assert "probing.ext.example" in sys.modules