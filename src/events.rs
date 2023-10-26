//! # Events specification.

use async_channel::{self as channel, Receiver, Sender, TrySendError};
use pin_project::pin_project;

mod payload;

pub use self::payload::EventType;

/// Event channel.
#[derive(Debug, Clone)]
pub struct Events {
    receiver: Receiver<Event>,
    sender: Sender<Event>,
}

impl Default for Events {
    fn default() -> Self {
        Self::new()
    }
}

impl Events {
    /// Creates a new event channel.
    pub fn new() -> Self {
        let (sender, receiver) = channel::bounded(1_000);

        Self { receiver, sender }
    }

    /// Emits an event into event channel.
    ///
    /// If the channel is full, deletes the oldest event first.
    pub fn emit(&self, event: Event) {
        match self.sender.try_send(event) {
            Ok(()) => {}
            Err(TrySendError::Full(event)) => {
                // when we are full, we pop remove the oldest event and push on the new one
                let _ = self.receiver.try_recv();

                // try again
                self.emit(event);
            }
            Err(TrySendError::Closed(_)) => {
                unreachable!("unable to emit event, channel disconnected");
            }
        }
    }

    /// Creates an event emitter.
    pub fn get_emitter(&self) -> EventEmitter {
        EventEmitter(self.receiver.clone())
    }
}

/// A receiver of events from a [`Context`].
///
/// See [`Context::get_event_emitter`] to create an instance.  If multiple instances are
/// created events emitted by the [`Context`] will only be delivered to one of the
/// `EventEmitter`s.
///
/// The `EventEmitter` is also a [`Stream`], so a typical usage is in a `while let` loop.
///
/// [`Context`]: crate::context::Context
/// [`Context::get_event_emitter`]: crate::context::Context::get_event_emitter
/// [`Stream`]: futures::stream::Stream
#[derive(Debug, Clone)]
#[pin_project]
pub struct EventEmitter(#[pin] Receiver<Event>);

impl EventEmitter {
    /// Async recv of an event. Return `None` if the `Sender` has been dropped.
    pub async fn recv(&self) -> Option<Event> {
        self.0.recv().await.ok()
    }
}

impl futures::stream::Stream for EventEmitter {
    type Item = Event;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.project().0.poll_next(cx)
    }
}

/// The event emitted by a [`Context`] from an [`EventEmitter`].
///
/// Events are documented on the C/FFI API in `deltachat.h` as `DC_EVENT_*` constants.  The
/// context emits them in relation to various operations happening, a lot of these are again
/// documented in `deltachat.h`.
///
/// [`Context`]: crate::context::Context
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// The ID of the [`Context`] which emitted this event.
    ///
    /// This allows using multiple [`Context`]s in a single process as they are identified
    /// by this ID.
    ///
    /// [`Context`]: crate::context::Context
    pub id: u32,
    /// The event payload.
    ///
    /// These are documented in `deltachat.h` as the `DC_EVENT_*` constants.
    pub typ: EventType,
}
