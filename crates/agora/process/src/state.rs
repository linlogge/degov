use std::{collections::HashMap, sync::Arc};
use std::ops::DerefMut;

use anyhow::Result;
use dgv_core::hash_map_id::HashMapId;
use tokio::sync::{
    Mutex, RwLock,
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};
use wasmtime::{ResourceLimiter, Table, component::Linker};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

use crate::{
    Signal, WasmtimeRuntime,
    config::{DefaultProcessConfig, ProcessConfig},
    env::{DegovEnvironment, Environment},
    mailbox::MessageMailbox,
    message::Message,
    runtime::wasmtime::WasmtimeCompiledComponent,
};

pub type ConfigResources<T> = HashMapId<T>;
pub type SignalSender = UnboundedSender<Signal>;
pub type SignalReceiver = Arc<Mutex<UnboundedReceiver<Signal>>>;

/// The internal state of a process.
///
/// The `ProcessState` has two main roles:
/// - It holds onto all vm resources (file descriptors, tcp streams, channels, ...)
/// - Registers all host functions working on those resources to the `Linker`
pub trait ProcessState: Sized + WasiView {
    type Config: ProcessConfig + Default + Send + Sync;

    // Create a new `ProcessState` using the parent's state (self) to inherit environment and
    // other parts of the state.
    // This is used in the guest function `spawn` which uses this trait and not the concrete state.
    fn new_state(
        &self,
        component: Arc<WasmtimeCompiledComponent<Self>>,
        config: Arc<Self::Config>,
    ) -> Result<Self>;

    fn register(linker: &mut Linker<Self>) -> Result<()>;

    /// Marks a wasm instance as initialized
    fn initialize(&mut self);
    /// Returns true if the instance was initialized
    fn is_initialized(&self) -> bool;

    /// Returns the WebAssembly runtime
    fn runtime(&self) -> &WasmtimeRuntime;
    // Returns the WebAssembly module
    fn component(&self) -> &Arc<WasmtimeCompiledComponent<Self>>;
    /// Returns the process configuration
    fn config(&self) -> &Arc<Self::Config>;

    // Returns process ID
    fn id(&self) -> u64;
    // Returns signal mailbox
    fn signal_mailbox(&self) -> &(SignalSender, SignalReceiver);
    // Returns message mailbox
    fn message_mailbox(&self) -> &MessageMailbox;

    // Config resources
    fn config_resources(&self) -> &ConfigResources<Self::Config>;
    fn config_resources_mut(&mut self) -> &mut ConfigResources<Self::Config>;

    // Registry
    fn registry(&self) -> &Arc<RwLock<HashMap<String, (u64, u64)>>>;
}

pub struct DefaultProcessState {
    // Process id
    pub(crate) id: u64,
    pub(crate) environment: Arc<DegovEnvironment>,
    // The WebAssembly runtime
    runtime: Option<WasmtimeRuntime>,
    // The component that this process was spawned from
    component: Option<Arc<WasmtimeCompiledComponent<Self>>>,
    // The process configuration
    config: Arc<DefaultProcessConfig>,
    // A space that can be used to temporarily store messages when sending or receiving them.
    // Messages can contain resources that need to be added across multiple host. Likewise,
    // receiving messages is done in two steps, first the message size is returned to allow the
    // guest to reserve enough space, and then it's received. Both of those actions use
    // `message` as a temp space to store messages across host calls.
    message: Option<Message>,
    // Signals sent to the mailbox
    signal_mailbox: (SignalSender, SignalReceiver),
    // Messages sent to the process
    message_mailbox: MessageMailbox,
    // Set to true if the WASM module has been instantiated
    initialized: bool,
    registry: Arc<RwLock<HashMap<String, (u64, u64)>>>,
    wasi: std::sync::Mutex<WasiCtx>,
    table: std::sync::Mutex<ResourceTable>,
}

impl DefaultProcessState {
    pub fn new(
        environment: Arc<DegovEnvironment>,
        runtime: WasmtimeRuntime,
        component: Arc<WasmtimeCompiledComponent<Self>>,
        config: Arc<DefaultProcessConfig>,
    ) -> Result<Self> {
        let signal_mailbox = unbounded_channel();
        let signal_mailbox = (signal_mailbox.0, Arc::new(Mutex::new(signal_mailbox.1)));
        let message_mailbox = MessageMailbox::default();
        let state = Self {
            id: environment.get_next_process_id(),
            environment,
            runtime: Some(runtime),
            component: Some(component),
            config: config.clone(),
            message: None,
            signal_mailbox,
            message_mailbox,
            initialized: false,
            registry: Arc::new(RwLock::new(HashMap::new())),
            wasi: std::sync::Mutex::new(WasiCtxBuilder::new().inherit_stdio().build()),
            table: std::sync::Mutex::new(ResourceTable::default()),
        };
        Ok(state)
    }
}

impl WasiView for DefaultProcessState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        // get_mut() is safe here because we have &mut self, guaranteeing exclusive access
        WasiCtxView {
            ctx: self.wasi.get_mut().unwrap(),
            table: self.table.get_mut().unwrap(),
        }
    }
}

impl ProcessState for DefaultProcessState {
    type Config = DefaultProcessConfig;
    
    fn new_state(
        &self,
        component: Arc<WasmtimeCompiledComponent<Self>>,
        config: Arc<Self::Config>,
    ) -> Result<Self> {
        todo!()
    }

    fn register(linker: &mut Linker<Self>) -> Result<()> {
        wasmtime_wasi::p2::add_to_linker_async(linker)?;
        Ok(())
    }
    
    fn initialize(&mut self) {
        self.initialized = true;
    }
    
    fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    fn runtime(&self) -> &WasmtimeRuntime {
        self.runtime.as_ref().unwrap()
    }
    
    fn component(&self) -> &Arc<WasmtimeCompiledComponent<Self>> {
        self.component.as_ref().unwrap()
    }
    
    fn config(&self) -> &Arc<Self::Config> {
        &self.config
    }
    
    fn id(&self) -> u64 {
        self.id
    }
    
    fn signal_mailbox(&self) -> &(SignalSender, SignalReceiver) {
        &self.signal_mailbox
    }
    
    fn message_mailbox(&self) -> &MessageMailbox {
        &self.message_mailbox
    }
    
    fn config_resources(&self) -> &ConfigResources<Self::Config> {
        todo!()
    }
    
    fn config_resources_mut(&mut self) -> &mut ConfigResources<Self::Config> {
        todo!()
    }
    
    fn registry(&self) -> &Arc<RwLock<HashMap<String, (u64, u64)>>> {
        &self.registry
    }
}

impl ResourceLimiter for DefaultProcessState {
    fn memory_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool> {
        Ok(desired <= self.config().get_max_memory())
    }

    fn table_growing(&mut self, _current: usize, desired: usize, _maximum: Option<usize>) -> Result<bool> {
        Ok(desired < 100_000)
    }

    // Allow multiple instances per store (needed for component model composition)
    fn instances(&self) -> usize {
        10
    }

    // Allow multiple tables per store (needed for component model composition)
    fn tables(&self) -> usize {
        10
    }

    // Allow one memory per store
    fn memories(&self) -> usize {
        1
    }
}

