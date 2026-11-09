//! The event bus: the engine publishes, everything else listens (app UI,
//! web server, database writer). Modules never call each other directly.
//!
//! Serialized events are also the WebSocket message format: a JSON object
//! with a `type` field (`"status"` or `"playFinished"`) and the payload's
//! fields beside it.

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::play::PlayRecord;
use crate::status::Status;

/// Room for bursts; a listener that falls further behind sees `Lagged` and
/// skips ahead. Status is a full snapshot, so skipping some is harmless.
pub const BUS_CAPACITY: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    /// Snapshot of everything shown live (at most 10 Hz).
    Status(Status),
    /// A play ended with at least one judgement; written to `data.db`.
    PlayFinished(PlayRecord),
}

pub type Bus = broadcast::Sender<Event>;

pub fn bus() -> Bus {
    broadcast::channel(BUS_CAPACITY).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_serialize_with_a_type_tag() {
        let json = serde_json::to_value(Event::Status(Status::default())).unwrap();
        assert_eq!(json["type"], "status");
        assert_eq!(json["connection"], "starting");
        assert_eq!(json["todayNotes"], 0);
    }
}
