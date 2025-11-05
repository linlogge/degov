use uniffi::export;

uniffi::setup_scaffolding!();

// Free function
#[uniffi::export]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
