// This conversion should always succeed.
pub fn u64_from_usize(num: usize) -> u64 {
    u64::try_from(num).expect("Failed converting a usize into a u64.")
}

#[cfg(test)]
mod tests {
    use super::*;

    use xpct::{equal, expect};

    #[test]
    fn convert_int() {
        expect!(u64_from_usize(42)).to(equal(42));
    }
}
