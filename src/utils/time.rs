/// Get current Unix timestamp in seconds.
/// Uses JavaScript Date API in Wasm/Workers environment.
/// Falls back to SystemTime for native tests.
pub fn now_timestamp() -> i64 {
    #[cfg(target_arch = "wasm32")]
    {
        use worker::js_sys;
        let now_ms = js_sys::Date::now();
        (now_ms / 1000.0) as i64
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_timestamp_returns_positive() {
        let timestamp = now_timestamp();
        assert!(timestamp > 0);
    }

    #[test]
    fn test_now_timestamp_is_reasonable() {
        let timestamp = now_timestamp();
        // Should be after 2020-01-01 (1577836800) and before 2030-01-01 (1893456000)
        assert!(timestamp > 1577836800);
        assert!(timestamp < 1893456000);
    }

    #[test]
    fn test_now_timestamp_increases() {
        let ts1 = now_timestamp();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let ts2 = now_timestamp();
        // Timestamps should be close (within 1 second for this test)
        assert!(ts2 >= ts1);
        assert!(ts2 - ts1 < 2);
    }
}
