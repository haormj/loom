use axum::{
    extract::State,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse,
    },
};
use futures_util::stream::{self, Stream};
use std::convert::Infallible;
use std::time::Duration;

use crate::routes::AppState;
use crate::watcher::DashboardEvent;

pub async fn sse_handler(State(state): State<AppState>) -> axum::response::Response {
    let rx = if let Some(rx) = state.event_rx.lock().await.as_mut() {
        rx.resubscribe()
    } else {
        return Sse::new(stream::empty::<Result<Event, Infallible>>())
            .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
            .into_response();
    };

    let stream = stream_channel(rx);

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

fn stream_channel(
    mut rx: tokio::sync::broadcast::Receiver<DashboardEvent>,
) -> impl Stream<Item = Result<Event, Infallible>> + Send {
    async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    yield Ok(Event::default()
                        .event(event.event_type.as_str())
                        .data(serde_json::to_string(&event.data).unwrap_or_default()));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    }
}
