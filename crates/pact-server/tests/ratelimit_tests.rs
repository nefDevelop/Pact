use pact_server::auth::ratelimit::RateLimiter;
use std::time::Duration;

#[test]
fn test_check_allows_under_limit() {
    let rl = RateLimiter::new(3, 60);
    assert!(rl.check("key1"));
    assert!(rl.check("key1"));
}

#[test]
fn test_check_blocks_at_limit() {
    let rl = RateLimiter::new(2, 60);
    assert!(rl.check("key1"));
    assert!(rl.check("key1"));
    assert!(!rl.check("key1"));
}

#[test]
fn test_remaining_decrements_correctly() {
    let rl = RateLimiter::new(3, 60);
    assert_eq!(rl.remaining("key1"), 3);

    rl.check("key1");
    assert_eq!(rl.remaining("key1"), 2);

    rl.check("key1");
    assert_eq!(rl.remaining("key1"), 1);

    rl.check("key1");
    assert_eq!(rl.remaining("key1"), 0);
}

#[test]
fn test_window_expiry_allows_again() {
    let rl = RateLimiter::new(2, 1);

    assert!(rl.check("key1"));
    assert!(rl.check("key1"));
    assert!(!rl.check("key1"));

    std::thread::sleep(Duration::from_secs(2));

    assert!(rl.check("key1"));
}

#[test]
fn test_cleanup_expired_removes_old_entries() {
    let rl = RateLimiter::new(5, 1);

    rl.check("key1");
    rl.check("key1");
    rl.check("key2");

    std::thread::sleep(Duration::from_secs(2));

    rl.cleanup_expired();

    assert_eq!(rl.remaining("key1"), 5);
    assert_eq!(rl.remaining("key2"), 5);
}

#[test]
fn test_different_keys_independent() {
    let rl = RateLimiter::new(2, 60);

    assert!(rl.check("alice"));
    assert!(rl.check("alice"));
    assert!(!rl.check("alice"));

    assert!(rl.check("bob"));
    assert!(rl.check("bob"));
    assert!(!rl.check("bob"));
}
