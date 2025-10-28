/// Test file to verify LogBroadcaster capacity guard against zero values
///
/// This test verifies that the LogBroadcaster::new() constructor properly
/// guards against zero capacity by clamping it to at least 1, preventing
/// the panic that would occur from broadcast::channel(0).

#[cfg(test)]
mod tests {
    use rcs::utils::ws_logger::LogBroadcaster;

    #[test]
    fn test_broadcaster_zero_capacity_guard() {
        // This test verifies that passing 0 capacity does NOT panic
        // The constructor should clamp it to 1 instead
        // If this test runs without panicking, the guard is working
        let broadcaster = LogBroadcaster::new(0);
        
        // Verify it works by sending a message
        broadcaster.send("test message".to_string());
        
        // Verify we can subscribe
        let _receiver = broadcaster.subscribe();
        
        // Verify basic functionality with the guarded capacity
        broadcaster.send("hello".to_string());
    }

    #[test]
    fn test_broadcaster_normal_capacity() {
        // Normal case should work as before
        let broadcaster = LogBroadcaster::new(100);
        broadcaster.send("test".to_string());
        let receiver = broadcaster.subscribe();
        assert!(!receiver.is_closed());
    }

    #[test]
    fn test_broadcaster_one_capacity() {
        // Edge case: explicit capacity of 1 should work
        let broadcaster = LogBroadcaster::new(1);
        broadcaster.send("test".to_string());
        let receiver = broadcaster.subscribe();
        assert!(!receiver.is_closed());
    }

    #[test]
    fn test_broadcaster_capacity_clamping() {
        // Verify that various small capacities are handled correctly
        // The key point: capacity 0 should not panic
        for capacity in [0, 1, 2, 5, 10] {
            let broadcaster = LogBroadcaster::new(capacity);
            broadcaster.send("message".to_string());
            let receiver = broadcaster.subscribe();
            assert!(!receiver.is_closed(), "Broadcaster with capacity {} should work", capacity);
        }
    }
}
