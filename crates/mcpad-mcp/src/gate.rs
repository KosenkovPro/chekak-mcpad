//! The confirmation of a change an AI agent would like to make.
//!
//! MCPad never writes anything on the request of a client without the user
//! seeing it first. The gate is the place where that promise is kept: the
//! server asks the gate, the gate asks the user, and only an explicit "yes"
//! lets the change happen.

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use serde::Serialize;

/// The longest preview of a new content that is shown to the user.
pub const PREVIEW_LIMIT: usize = 400;

/// A change a client would like to make.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteRequest {
    /// The identifier of the request, the answer is given with it.
    pub id: String,
    /// The tool that asks for the change.
    pub tool: String,
    /// The path of the file, relative to the vault.
    pub path: String,
    /// A short description for the user.
    pub summary: String,
    /// The beginning of the new content, for the preview.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

impl WriteRequest {
    /// Builds a request for a change of a file.
    pub fn new(
        id: impl Into<String>,
        tool: impl Into<String>,
        path: impl Into<String>,
        summary: impl Into<String>,
        preview: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            tool: tool.into(),
            path: path.into(),
            summary: summary.into(),
            preview,
        }
    }
}

/// The answer of the user.
#[derive(Debug, Clone)]
pub enum Decision {
    /// The change may be made.
    Allow,
    /// The change is refused, with the reason for the client.
    Deny(String),
}

/// Decides whether a change may be made.
pub trait Gate: Send + Sync {
    /// Decides about a change. The call may wait until the user answered.
    fn decide(&self, request: WriteRequest) -> Decision;
}

/// A gate that refuses every change.
///
/// This is the mode MCPad starts in: an AI agent may look at the vault, but
/// it does not touch anything.
#[derive(Debug, Clone, Copy, Default)]
pub struct DenyGate;

impl Gate for DenyGate {
    fn decide(&self, _request: WriteRequest) -> Decision {
        Decision::Deny(
            "the server works in the read only mode, the user has not allowed any changes"
                .to_string(),
        )
    }
}

/// A gate that allows every change.
///
/// It is only used when the user starts the server with the explicit flag
/// that allows changes, for example from a script of their own.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllowGate;

impl Gate for AllowGate {
    fn decide(&self, _request: WriteRequest) -> Decision {
        Decision::Allow
    }
}

/// A gate that waits for the user to confirm the change in the application.
pub struct ChannelGate {
    timeout: Duration,
    inner: Mutex<Inner>,
    listener: Mutex<Option<Sender<WriteRequest>>>,
}

/// The requests that wait for an answer.
struct Inner {
    pending: HashMap<String, (WriteRequest, Sender<Decision>)>,
}

impl ChannelGate {
    /// Creates a gate that waits at most `timeout` for the user.
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            inner: Mutex::new(Inner {
                pending: HashMap::new(),
            }),
            listener: Mutex::new(None),
        }
    }

    /// Registers the channel that receives the requests to confirm.
    pub fn set_listener(&self, sender: Sender<WriteRequest>) {
        *lock(&self.listener) = Some(sender);
    }

    /// The number of the requests that wait for an answer.
    pub fn pending_count(&self) -> usize {
        lock(&self.inner).pending.len()
    }

    /// The requests that wait for an answer.
    pub fn pending(&self) -> Vec<WriteRequest> {
        lock(&self.inner)
            .pending
            .values()
            .map(|(request, _)| request.clone())
            .collect()
    }

    /// Answers a request.
    ///
    /// Returns `false` when the request is unknown, for example because the
    /// user was too slow and the request timed out already.
    pub fn resolve(&self, id: &str, decision: Decision) -> bool {
        let entry = lock(&self.inner).pending.remove(id);
        match entry {
            Some((_, sender)) => sender.send(decision).is_ok(),
            None => false,
        }
    }
}

impl Gate for ChannelGate {
    fn decide(&self, request: WriteRequest) -> Decision {
        let (sender, receiver) = channel();
        {
            let mut inner = lock(&self.inner);
            inner
                .pending
                .insert(request.id.clone(), (request.clone(), sender));
        }

        let listener = lock(&self.listener).clone();
        match listener {
            Some(listener) => {
                if listener.send(request.clone()).is_err() {
                    lock(&self.inner).pending.remove(&request.id);
                    return Decision::Deny("nobody can confirm the change any more".to_string());
                }
            }
            None => {
                lock(&self.inner).pending.remove(&request.id);
                return Decision::Deny(
                    "nobody can confirm the change, there is no one watching".to_string(),
                );
            }
        }

        let decision = match receiver.recv_timeout(self.timeout) {
            Ok(decision) => decision,
            Err(RecvTimeoutError::Timeout) => {
                Decision::Deny("the user did not confirm the change in time".to_string())
            }
            Err(RecvTimeoutError::Disconnected) => {
                Decision::Deny("the confirmation channel is closed".to_string())
            }
        };

        lock(&self.inner).pending.remove(&request.id);
        decision
    }
}

/// Locks a mutex and ignores a poisoned lock.
///
/// A panic inside the server must not lock the user out of the application,
/// therefore the data is taken out of the mutex even when it is poisoned.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

/// Receives the requests to confirm.
pub type RequestReceiver = Receiver<WriteRequest>;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[test]
    fn the_read_only_gate_refuses_every_change() {
        let request = WriteRequest::new("1", "write_file", "a.md", "write a.md", None);
        match DenyGate.decide(request) {
            Decision::Deny(reason) => assert!(reason.contains("read only")),
            Decision::Allow => panic!("the read only gate must not allow a change"),
        }
    }

    #[test]
    fn the_open_gate_allows_every_change() {
        let request = WriteRequest::new("1", "write_file", "a.md", "write a.md", None);
        assert!(matches!(AllowGate.decide(request), Decision::Allow));
    }

    #[test]
    fn the_user_can_confirm_a_change() {
        let gate = Arc::new(ChannelGate::new(Duration::from_secs(5)));
        let (sender, receiver) = channel();
        gate.set_listener(sender);

        let request = WriteRequest::new("1", "write_file", "a.md", "write a.md", None);
        let worker = Arc::clone(&gate);
        let answer = std::thread::spawn(move || worker.decide(request));

        let pending = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(pending.path, "a.md");
        assert_eq!(gate.pending_count(), 1);

        assert!(gate.resolve("1", Decision::Allow));
        assert!(matches!(answer.join().unwrap(), Decision::Allow));
        assert_eq!(gate.pending_count(), 0);
    }

    #[test]
    fn the_user_can_refuse_a_change() {
        let gate = Arc::new(ChannelGate::new(Duration::from_secs(5)));
        let (sender, receiver) = channel();
        gate.set_listener(sender);

        let request = WriteRequest::new("1", "delete_file", "a.md", "delete a.md", None);
        let worker = Arc::clone(&gate);
        let answer = std::thread::spawn(move || worker.decide(request));

        receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(gate.resolve("1", Decision::Deny("not now".to_string())));

        match answer.join().unwrap() {
            Decision::Deny(reason) => assert_eq!(reason, "not now"),
            Decision::Allow => panic!("the change must not be allowed"),
        }
    }

    #[test]
    fn a_change_that_is_not_confirmed_in_time_is_refused() {
        let gate = ChannelGate::new(Duration::from_millis(50));
        let (sender, _receiver) = channel();
        gate.set_listener(sender);

        let request = WriteRequest::new("1", "write_file", "a.md", "write a.md", None);
        match gate.decide(request) {
            Decision::Deny(reason) => assert!(reason.contains("in time")),
            Decision::Allow => panic!("the change must not be allowed"),
        }
        assert_eq!(gate.pending_count(), 0);
    }

    #[test]
    fn a_change_without_anybody_to_confirm_it_is_refused() {
        let gate = ChannelGate::new(Duration::from_millis(50));
        let request = WriteRequest::new("1", "write_file", "a.md", "write a.md", None);
        match gate.decide(request) {
            Decision::Deny(reason) => assert!(reason.contains("nobody")),
            Decision::Allow => panic!("the change must not be allowed"),
        }
    }
}
