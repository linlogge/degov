use anyhow::Result;
use smallvec::SmallVec;
use tokio::sync::Mutex;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tracing::{debug, trace, warn};

pub mod config;
pub mod env;
mod mailbox;
mod message;
pub mod runtime;
pub mod state;
pub mod wasm;

use crate::env::Environment;
use crate::mailbox::MessageMailbox;
use crate::message::Message;
use crate::state::ProcessState;

use runtime::wasmtime::WasmtimeRuntime;

/// The `Process` is the main abstraction in lunatic.
///
/// It usually represents some code that is being executed (Wasm instance or V8 isolate), but it
/// could also be a resource (GPU, UDP connection) that can be interacted with through messages.
///
/// The only way of interacting with them is through signals. These signals can come in different
/// shapes (message, kill, link, ...). Most signals have well defined meanings, but others such as
/// a [`Message`] are opaque and left to the receiver for interpretation.
pub trait Process: Send + Sync {
    fn id(&self) -> u64;
    fn send(&self, signal: Signal);
}

impl Debug for dyn Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Point").field("id", &self.id()).finish()
    }
}

impl Hash for dyn Process {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

/// Signals can be sent to processes to interact with them.
pub enum Signal {
    // Messages can contain opaque data.
    Message(Message),
    // When received, the process should stop immediately.
    Kill,
    // Change behaviour of what happens if a linked process dies.
    DieWhenLinkDies(bool),
    // Sent from a process that wants to be linked. In case of a death the tag will be returned
    // to the sender in form of a `LinkDied` signal.
    Link(Option<i64>, Arc<dyn Process>),
    // Request from a process to be unlinked
    UnLink { process_id: u64 },
    // Sent to linked processes when the link dies. Contains the tag used when the link was
    // established. Depending on the value of `die_when_link_dies` (default is `true`) and
    // the death reason, the receiving process will turn this signal into a message or the
    // process will immediately die as well.
    LinkDied(u64, Option<i64>, DeathReason),
    Monitor(Arc<dyn Process>),
    StopMonitoring { process_id: u64 },
    ProcessDied(u64),
}

impl Debug for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Message(_) => write!(f, "Message"),
            Self::Kill => write!(f, "Kill"),
            Self::DieWhenLinkDies(_) => write!(f, "DieWhenLinkDies"),
            Self::Link(_, p) => write!(f, "Link {}", p.id()),
            Self::UnLink { process_id } => write!(f, "UnLink {process_id}"),
            Self::LinkDied(_, _, reason) => write!(f, "LinkDied {reason:?}"),
            Self::Monitor(p) => write!(f, "Monitor {}", p.id()),
            Self::StopMonitoring { process_id } => write!(f, "UnMonitor {process_id}"),
            Self::ProcessDied(_) => write!(f, "ProcessDied"),
        }
    }
}

// The reason of a process' death
#[derive(Clone, Copy, Debug)]
pub enum DeathReason {
    // Process finished normaly.
    Normal,
    Failure,
    NoProcess,
}

/// The reason of a process finishing
pub enum Finished<T> {
    /// This just means that the process finished without external interaction.
    /// In case of Wasm this could mean that the entry function returned normally or that it
    /// **trapped**.
    Normal(T),
    /// The process was terminated by an external `Kill` signal.
    KillSignal,
}

/// A `WasmProcess` represents an instance of a Wasm module that is being executed.
///
/// They can be created with [`spawn_wasm`](crate::wasm::spawn_wasm), and once spawned they will be
/// running in the background and can't be observed directly.
#[derive(Debug, Clone)]
pub struct WasmProcess {
    id: u64,
    signal_mailbox: UnboundedSender<Signal>,
}

impl WasmProcess {
    /// Create a new WasmProcess
    pub fn new(id: u64, signal_mailbox: UnboundedSender<Signal>) -> Self {
        Self { id, signal_mailbox }
    }
}

impl Process for WasmProcess {
    fn id(&self) -> u64 {
        self.id
    }

    fn send(&self, signal: Signal) {
        // If the receiver doesn't exist or is closed, just ignore it and drop the `signal`.
        // lunatic can't guarantee that a message was successfully seen by the receiving side even
        // if this call succeeds. We deliberately don't expose this API, as it would not make sense
        // to relay on it and could signal wrong guarantees to users.
        let _ = self.signal_mailbox.send(signal);
    }
}

/// Enum containing a process name if available, otherwise its ID.
enum NameOrID<'a> {
    Names(SmallVec<[&'a str; 2]>),
    ID(u64),
}

impl<'a> NameOrID<'a> {
    /// Returns names, otherwise id if names is empty.
    fn or_id(self, id: u64) -> Self {
        match self {
            NameOrID::Names(ref names) if !names.is_empty() => self,
            _ => NameOrID::ID(id),
        }
    }
}

impl<'a> std::fmt::Display for NameOrID<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameOrID::Names(names) => {
                for (i, name) in names.iter().enumerate() {
                    if i > 0 {
                        write!(f, " / ")?;
                    }
                    write!(f, "'{name}'")?;
                }
                Ok(())
            }
            NameOrID::ID(id) => write!(f, "{id}"),
        }
    }
}

impl<'a> FromIterator<&'a str> for NameOrID<'a> {
    fn from_iter<T: IntoIterator<Item = &'a str>>(iter: T) -> Self {
        let names = SmallVec::from_iter(iter);
        NameOrID::Names(names)
    }
}

// Contains the result of a process execution.
//
// Can be also used to extract the state of a process after the execution is done.
pub struct ExecutionResult<T> {
    state: T,
    result: ResultValue,
}

impl<T> ExecutionResult<T> {
    // Returns the failure as `String` if the process failed.
    pub fn failure(&self) -> Option<&str> {
        match self.result {
            ResultValue::Failed(ref failure) => Some(failure),
            ResultValue::SpawnError(ref failure) => Some(failure),
            _ => None,
        }
    }

    // Returns the process state reference
    pub fn state(&self) -> &T {
        &self.state
    }

    // Returns the process state
    pub fn into_state(self) -> T {
        self.state
    }
}

// It's more convinient to return a `Result<T,E>` in a `NativeProcess`.
impl<T> From<Result<T>> for ExecutionResult<T>
where
    T: Default,
{
    fn from(result: Result<T>) -> Self {
        match result {
            Ok(t) => ExecutionResult {
                state: t,
                result: ResultValue::Ok,
            },
            Err(e) => ExecutionResult {
                state: T::default(),
                result: ResultValue::Failed(e.to_string()),
            },
        }
    }
}

pub enum ResultValue {
    Ok,
    Failed(String),
    SpawnError(String),
}

/// Turns a `Future` into a process, enabling signals (e.g. kill).
///
/// This function represents the core execution loop of lunatic processes:
///
/// 1. The process will first check if there are any new signals and handle them.
/// 2. If no signals are available, it will poll the `Future` and advance the execution.
///
/// This steps are repeated until the `Future` returns `Poll::Ready`, indicating the end of the
/// computation.
///
/// The `Future` is in charge to periodically yield back the execution with `Poll::Pending` to give
/// the signal handler a chance to run and process pending signals.
///
/// In case of success, the process state `S` is returned. It's not possible to return the process
/// state in case of failure because of limitations in the Wasmtime API:
/// https://github.com/bytecodealliance/wasmtime/issues/2986
pub(crate) async fn new<F, S, R>(
    fut: F,
    id: u64,
    env: Arc<dyn Environment>,
    signal_mailbox: Arc<Mutex<UnboundedReceiver<Signal>>>,
    message_mailbox: MessageMailbox,
) -> Result<S>
where
    S: ProcessState,
    R: Into<ExecutionResult<S>>,
    F: Future<Output = R> + Send + 'static,
{
    trace!("Process {} spawned", id);
    tokio::pin!(fut);

    // Defines what happens if one of the linked processes dies.
    // If the value is set to false, instead of dying too the process will receive a message about
    // the linked process' death.
    let mut die_when_link_dies = true;
    // Process linked to this one
    let mut links = HashMap::new();
    // Processes monitoring this one
    let mut monitors = HashMap::new();
    // TODO: Maybe wrapping this in some kind of `std::panic::catch_unwind` wold be a good idea,
    //       to protect against panics in host function calls that unwind through Wasm code.
    //       Currently a panic would just kill the task, but not notify linked processes.
    let mut signal_mailbox = signal_mailbox.lock().await;
    let mut has_sender = true;

    let result = loop {
        tokio::select! {
            biased;
            // Handle signals first
            signal = signal_mailbox.recv(), if has_sender => {

                match signal.ok_or(()) {
                    Ok(Signal::Message(message)) => {
                        message_mailbox.push(message);
                    },
                    Ok(Signal::DieWhenLinkDies(value)) => die_when_link_dies = value,
                    // Put process into list of linked processes
                    Ok(Signal::Link(tag, proc)) => {
                        links.insert(proc.id(), (proc, tag));
                    },
                    // Remove process from list
                    Ok(Signal::UnLink { process_id }) => {
                        links.remove(&process_id);
                    }
                    // Exit loop and don't poll anymore the future if Signal::Kill received.
                    Ok(Signal::Kill) => break Finished::KillSignal,
                    // Depending if `die_when_link_dies` is set, process will die or turn the
                    // signal into a message
                    Ok(Signal::LinkDied(id, tag, reason)) => {
                        links.remove(&id);
                        match reason {
                            DeathReason::Failure | DeathReason::NoProcess => {
                                if die_when_link_dies {
                                    // Even this was not a **kill** signal it has the same effect on
                                    // this process and should be propagated as such.
                                    break Finished::KillSignal
                                } else {
                                    let message = Message::LinkDied(tag);
                                    message_mailbox.push(message);
                                }
                            },
                            // In case a linked process finishes normally, don't do anything.
                            DeathReason::Normal => {},
                        }
                    },
                    // Put process into list of monitor processes
                    Ok(Signal::Monitor(proc)) => {
                        monitors.insert(proc.id(), proc);
                    }
                    // Remove process from monitor list
                    Ok(Signal::StopMonitoring { process_id }) => {
                        monitors.remove(&process_id);
                    }
                    // Notify process that a monitored process died
                    Ok(Signal::ProcessDied(id)) => {
                        message_mailbox.push(Message::ProcessDied(id));
                    }
                    Err(_) => {
                        debug_assert!(has_sender);
                        has_sender = false;
                    }
                }
            }
            // Run process
            output = &mut fut => { break Finished::Normal(output); }
        }
    };

    env.remove_process(id);

    let result = match result {
        Finished::Normal(result) => {
            let result: ExecutionResult<_> = result.into();

            if let Some(failure) = result.failure() {
                let registry = result.state().registry().read().await;
                let name = registry
                    .iter()
                    .filter(|(_, (_, process_id))| process_id == &id)
                    .map(|(name, _)| name.splitn(4, '/').last().unwrap_or(name.as_str()))
                    .collect::<NameOrID>()
                    .or_id(id);
                warn!("Process {} failed, notifying: {} links", name, links.len());
                debug!("{}", failure);

                Err(anyhow::anyhow!(failure.to_string()))
            } else {
                Ok(result.into_state())
            }
        }
        Finished::KillSignal => {
            warn!(
                "Process {} was killed, notifying: {} links",
                id,
                links.len()
            );

            Err(anyhow::anyhow!("Process received Kill signal"))
        }
    };

    let reason = match result {
        Ok(_) => DeathReason::Normal,
        Err(_) => DeathReason::Failure,
    };

    // Notify all links that we finished
    for (proc, tag) in links.values() {
        proc.send(Signal::LinkDied(id, *tag, reason));
    }

    // Notify all monitoring processes we died
    for proc in monitors.values() {
        proc.send(Signal::ProcessDied(id));
    }

    result
}
