#[no_mangle]
pub extern "C" fn double64u(input: u64) -> u64 {
    input << 1
}
