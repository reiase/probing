use axum::extract::ws::Message;
use futures_util::{SinkExt, StreamExt};
use probing_python::repl::Repl;

pub async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(|ws| async move {
        // Handle WebSocket connection
        log::info!("WebSocket connection established");
        let (mut write, mut read) = ws.split();

        let mut repl = probing_python::repl::PythonRepl::default();

        while let Some(Ok(msg)) = read.next().await {
            if let Message::Text(msg) = msg {
                let rsp = repl.feed(msg.to_string()).unwrap_or("{}".to_string());

                if write.send(Message::Text(rsp.into())).await.is_err() {
                    break;
                }
            }
        }
        // Here you would handle the WebSocket messages
        // For now, just close the connection
    })
}
