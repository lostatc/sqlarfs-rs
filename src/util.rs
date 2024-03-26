// This conversion should always succeed.
pub fn u64_from_usize(num: usize) -> u64 {
    u64::try_from(num).expect("Failed converting a usize into a u64.")
}
