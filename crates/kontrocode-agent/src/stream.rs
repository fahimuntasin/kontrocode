//! Streaming — the agent emits events to the UI as it works.

use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use kontrocode_core::{MessageId, ToolCall, ToolResult};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Every event the agent emits. Mapped to Tauri events by the shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Agent started processing a user message.
    Started {
        /// The user message id this run is responding to.
        message_id: MessageId,
    },
    /// A chunk of streamed text. Many per response.
    TextChunk {
        /// The assistant message id being streamed.
        message_id: MessageId,
        /// The incremental text.
        delta: String,
    },
    /// The model wants to call a tool.
    ToolCall {
        /// The assistant message id.
        message_id: MessageId,
        /// The tool call details.
        call: ToolCall,
    },
    /// A tool's execution result, ready to feed back to the model.
    ToolResult {
        /// The tool call id this result corresponds to.
        tool_call_id: String,
        /// The structured result.
        result: ToolResult,
    },
    /// Research feed update — what the agent is looking up.
    ResearchUpdate {
        /// Short title of what's being looked up.
        title: String,
        /// Free-form details.
        body: String,
    },
    /// The full response is done.
    Done {
        /// The final assistant message id.
        message_id: MessageId,
    },
    /// An error occurred.
    Error {
        /// Human-readable error message.
        message: String,
    },
}

/// A handle for sending events into a stream.
#[derive(Debug, Clone)]
pub struct StreamSender {
    tx: mpsc::UnboundedSender<AgentEvent>,
}

impl StreamSender {
    /// Send an event. Returns `false` if the receiver was dropped.
    pub fn send(&self, event: AgentEvent) -> bool {
        self.tx.send(event).is_ok()
    }

    /// Convenience: send a [`AgentEvent::TextChunk`].
    pub fn text_chunk(&self, message_id: MessageId, delta: impl Into<String>) {
        let _ = self.tx.send(AgentEvent::TextChunk {
            message_id,
            delta: delta.into(),
        });
    }

    /// Convenience: send a [`AgentEvent::Error`].
    pub fn error(&self, message: impl Into<String>) {
        let _ = self.tx.send(AgentEvent::Error {
            message: message.into(),
        });
    }
}

/// A receiver for agent events. The UI side of the stream.
#[derive(Debug)]
pub struct StreamReceiver {
    rx: mpsc::UnboundedReceiver<AgentEvent>,
}

impl StreamReceiver {
    /// Convert to a boxed stream.
    pub fn into_stream(self) -> Pin<Box<dyn Stream<Item = AgentEvent> + Send>> {
        let rx = self.rx;
        Box::pin(futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|event| (event, rx))
        }))
    }
}

/// Build a connected sender/receiver pair. The sender is cheap to clone.
pub fn channel() -> (StreamSender, StreamReceiver) {
    let (tx, rx) = mpsc::unbounded_channel();
    (StreamSender { tx }, StreamReceiver { rx })
}

/// Collect all events from a stream into a `Vec`. Used in tests.
pub async fn collect<S>(stream: S) -> Vec<AgentEvent>
where
    S: Stream<Item = AgentEvent> + Unpin,
{
    use futures::StreamExt;
    let mut out = Vec::new();
    let mut s = stream;
    while let Some(e) = s.next().await {
        out.push(e);
    }
    out
}

/// A shared buffer of the last N events. Useful for late-attaching UIs
/// (e.g. after a window reload).
#[derive(Debug, Clone)]
pub struct EventBuffer {
    inner: Arc<Mutex<Vec<AgentEvent>>>,
    capacity: usize,
}

impl EventBuffer {
    /// Construct a buffer that keeps the last `capacity` events.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::with_capacity(capacity))),
            capacity,
        }
    }

    /// Push an event. Evicts the oldest if at capacity.
    pub fn push(&self, event: AgentEvent) {
        let mut buf = self.inner.lock();
        if buf.len() >= self.capacity {
            buf.remove(0);
        }
        buf.push(event);
    }

    /// Snapshot the current contents.
    pub fn snapshot(&self) -> Vec<AgentEvent> {
        self.inner.lock().clone()
    }

    /// Clear all events.
    pub fn clear(&self) {
        self.inner.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sender_receiver_round_trip() {
        let (tx, rx) = channel();
        tx.text_chunk(MessageId::new(), "hello");
        tx.text_chunk(MessageId::new(), " world");
        drop(tx);
        let events = collect(rx.into_stream()).await;
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn buffer_evicts_oldest() {
        let b = EventBuffer::new(2);
        b.push(AgentEvent::Error {
            message: "1".into(),
        });
        b.push(AgentEvent::Error {
            message: "2".into(),
        });
        b.push(AgentEvent::Error {
            message: "3".into(),
        });
        let snap = b.snapshot();
        assert_eq!(snap.len(), 2);
    }

    #[test]
    fn buffer_clear_empties() {
        let b = EventBuffer::new(8);
        b.push(AgentEvent::Error {
            message: "x".into(),
        });
        b.clear();
        assert!(b.snapshot().is_empty());
    }
}
