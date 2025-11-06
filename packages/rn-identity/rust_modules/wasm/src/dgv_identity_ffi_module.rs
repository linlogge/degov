#[allow(unused_imports)]
use uniffi_runtime_javascript::{self as js, uniffi as u, IntoJs, IntoRust};
use wasm_bindgen::prelude::wasm_bindgen;
extern "C" {
    fn uniffi_dgv_identity_ffi_fn_func_add(
        a: i32,
        b: i32,
        status_: &mut u::RustCallStatus,
    ) -> i32;
    fn uniffi_dgv_identity_ffi_checksum_func_add() -> u16;
    fn ffi_dgv_identity_ffi_uniffi_contract_version() -> u32;
}
#[wasm_bindgen]
pub fn ubrn_uniffi_dgv_identity_ffi_fn_func_add(
    a: js::Int32,
    b: js::Int32,
    f_status_: &mut js::RustCallStatus,
) -> js::Int32 {
    let mut u_status_ = u::RustCallStatus::default();
    let value_ = unsafe {
        uniffi_dgv_identity_ffi_fn_func_add(
            i32::into_rust(a),
            i32::into_rust(b),
            &mut u_status_,
        )
    };
    f_status_.copy_from(u_status_);
    value_.into_js()
}
#[wasm_bindgen]
pub unsafe fn ubrn_uniffi_dgv_identity_ffi_checksum_func_add() -> js::UInt16 {
    uniffi_dgv_identity_ffi_checksum_func_add().into_js()
}
#[wasm_bindgen]
pub unsafe fn ubrn_ffi_dgv_identity_ffi_uniffi_contract_version() -> js::UInt32 {
    ffi_dgv_identity_ffi_uniffi_contract_version().into_js()
}
