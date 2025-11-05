use std::sync::Arc;

use anyhow::{Context, Result};
use wasmtime::{
    Engine, ResourceLimiter,
    component::{Component, Val},
};

use crate::{
    ExecutionResult, ResultValue, config::{ProcessConfig, UNIT_OF_COMPUTE_IN_INSTRUCTIONS}, runtime::RawWasm,
    state::ProcessState,
};

#[derive(Clone)]
pub struct WasmtimeRuntime {
    engine: wasmtime::Engine,
}

impl WasmtimeRuntime {
    pub fn try_new(config: &wasmtime::Config) -> Result<Self> {
        let engine = wasmtime::Engine::new(config)?;
        Ok(Self { engine })
    }

    pub fn compile_component<T>(&self, data: RawWasm) -> Result<WasmtimeCompiledComponent<T>>
    where
        T: ProcessState + 'static,
    {
        let component = Component::new(&self.engine, data.as_slice())?;
        let mut linker = wasmtime::component::Linker::new(&self.engine);
        T::register(&mut linker)?;

        let instance_pre = linker.instantiate_pre(&component)?;
        let compiled_component = WasmtimeCompiledComponent::new(data, component, instance_pre);
        Ok(compiled_component)
    }

    pub async fn instantiate<T>(
        &self,
        compiled_component: &WasmtimeCompiledComponent<T>,
        state: T,
    ) -> Result<WasmtimeInstance<T>>
    where
        T: ProcessState + Send + ResourceLimiter + 'static,
    {
        let max_fuel = state.config().get_max_fuel().unwrap_or(u64::MAX);
        let mut store = wasmtime::Store::new(&self.engine, state);
        // Set limits of the store
        store.limiter(|state| state);
        // Trap if out of fuel
        //store.set_fuel(max_fuel)?;

        // Create instance
        let instance = compiled_component
            .instantiator()
            .instantiate_async(&mut store)
            .await?;
        // Mark state as initialized
        store.data_mut().initialize();
        Ok(WasmtimeInstance { store, instance })
    }
}

pub struct WasmtimeCompiledComponent<T: 'static> {
    inner: Arc<WasmtimeCompiledModuleInner<T>>,
}

pub struct WasmtimeCompiledModuleInner<T: 'static> {
    source: RawWasm,
    component: wasmtime::component::Component,
    instance_pre: wasmtime::component::InstancePre<T>,
}

impl<T: 'static> WasmtimeCompiledComponent<T> {
    pub fn new(
        source: RawWasm,
        component: wasmtime::component::Component,
        instance_pre: wasmtime::component::InstancePre<T>,
    ) -> WasmtimeCompiledComponent<T> {
        let inner = Arc::new(WasmtimeCompiledModuleInner {
            source,
            component,
            instance_pre,
        });
        Self { inner }
    }

    pub fn source(&self) -> &RawWasm {
        &self.inner.source
    }

    pub fn instantiator(&self) -> &wasmtime::component::InstancePre<T> {
        &self.inner.instance_pre
    }
}

impl<T: 'static> Clone for WasmtimeCompiledComponent<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

pub struct WasmtimeInstance<T>
where
    T: Send + 'static,
{
    store: wasmtime::Store<T>,
    instance: wasmtime::component::Instance,
}

impl<T> WasmtimeInstance<T>
where
    T: Send,
{
    pub async fn call(
        mut self,
        function_str: &str,
        params: Vec<wasmtime::component::Val>,
    ) -> ExecutionResult<T> {
        use wasmtime::component::{
            Val,
            wasm_wave::{
                untyped::UntypedFuncCall,
                wasm::{DisplayFuncResults, WasmFunc},
            },
        };
        
        let func = match self.instance.get_func(&mut self.store, function_str) {
            Some(func) => func,
            None => {
                return ExecutionResult {
                    state: self.store.into_data(),
                    result: ResultValue::SpawnError(format!("Failed to get func '{function_str}'")),
                };
            }
        };

        let func_params = func.params(&self.store);
        let func_results = func.results(&self.store);

        let mut results = vec![Val::Bool(false); func_results.len()];
        let result = func
            .call_async(&mut self.store, &vec![Val::U32(1), Val::U32(2)], &mut results)
            .await;

        match result {
            Ok(()) => {
                println!("Result: {:?}", results);
            },
            Err(e) => {
                println!("Error calling func: {:?}", e);
                return ExecutionResult {
                    state: self.store.into_data(),
                    result: ResultValue::SpawnError(format!(
                        "Failed to call func '{function_str}': {e}"
                    )),
                };
            }
        }

        match func.post_return_async(&mut self.store).await {
            Ok(()) => (),
            Err(e) => {
                return ExecutionResult {
                    state: self.store.into_data(),
                    result: ResultValue::SpawnError(format!(
                        "Failed to post return func '{function_str}': {e}"
                    )),
                };
            }
        }

        ExecutionResult {
            state: self.store.into_data(),
            result: ResultValue::Ok,
        }
    }

    fn search_component_funcs(
        engine: &Engine,
        component: wasmtime::component::types::Component,
        name: &str,
    ) -> Vec<(Vec<String>, wasmtime::component::types::ComponentFunc)> {
        use wasmtime::component::types::ComponentItem as CItem;
        fn collect_exports(
            engine: &Engine,
            item: CItem,
            basename: Vec<String>,
        ) -> Vec<(Vec<String>, CItem)> {
            match item {
                CItem::Component(c) => c
                    .exports(engine)
                    .flat_map(move |(name, item)| {
                        let mut names = basename.clone();
                        names.push(name.to_string());
                        collect_exports(engine, item, names)
                    })
                    .collect::<Vec<_>>(),
                CItem::ComponentInstance(c) => c
                    .exports(engine)
                    .flat_map(move |(name, item)| {
                        let mut names = basename.clone();
                        names.push(name.to_string());
                        collect_exports(engine, item, names)
                    })
                    .collect::<Vec<_>>(),
                _ => vec![(basename, item)],
            }
        }

        collect_exports(engine, CItem::Component(component), Vec::new())
            .into_iter()
            .filter_map(|(names, item)| {
                let CItem::ComponentFunc(func) = item else {
                    return None;
                };
                let func_name = names.last().expect("at least one name");
                let base_func_name = func_name.strip_prefix("[async]").unwrap_or(func_name);
                (base_func_name == name).then_some((names, func))
            })
            .collect()
    }
}

pub fn default_config() -> wasmtime::Config {
    let mut config = wasmtime::Config::new();
    config
        .async_support(true)
        .debug_info(true)
        // The behavior of fuel running out is defined on the Store
        //.consume_fuel(true)
        .wasm_reference_types(true)
        .wasm_bulk_memory(true)
        .wasm_multi_value(true)
        .wasm_multi_memory(true)
        .cranelift_opt_level(wasmtime::OptLevel::SpeedAndSize)
        // Allocate resources on demand because we can't predict how many process will exist
        .allocation_strategy(wasmtime::InstanceAllocationStrategy::OnDemand);
    config
}
