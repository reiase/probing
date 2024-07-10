use crate::repl::PythonRepl;

pub fn handle(path: &str, query: Option<String>) -> String {
    let request = format!(
        "handle(path=\"{}\", query={})\n",
        path,
        query
            .map(|qs| { format!("\"{}\"", qs) })
            .unwrap_or("None".to_string())
    );
    let mut repl = PythonRepl::default();
    repl.process(request.as_str()).unwrap_or("".to_string())
}
