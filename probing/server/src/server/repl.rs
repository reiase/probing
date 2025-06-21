pub async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(|_ws| async move {
        // Handle WebSocket connection
        log::info!("WebSocket connection established");
        // Here you would handle the WebSocket messages
        // For now, just close the connection
    })
}