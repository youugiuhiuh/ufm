//! ufm-download crate
pub fn is_ready() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ready() {
        assert!(is_ready());
    }
}
