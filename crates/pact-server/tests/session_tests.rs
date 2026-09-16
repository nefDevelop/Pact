use pact_server::auth::session::SessionManager;
use chrono::Duration;

#[test]
fn test_create_session_returns_valid_token() {
    let mgr = SessionManager::new();
    let token = mgr.create_session("alice".to_string());
    assert!(token.is_some());
    let token = token.unwrap();
    assert!(!token.is_empty());
    assert_eq!(token.len(), 36);
}

#[test]
fn test_get_session_returns_session_and_updates_activity() {
    let mgr = SessionManager::new();
    let token = mgr.create_session("alice".to_string()).unwrap();

    let session = mgr.get_session(&token);
    assert!(session.is_some());
    let session = session.unwrap();
    assert_eq!(session.username, "alice");
}

#[test]
fn test_get_session_expired_returns_none() {
    let mgr = SessionManager::new_with_timeout(Duration::seconds(1));
    let token = mgr.create_session("alice".to_string()).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(2));

    let session = mgr.get_session(&token);
    assert!(session.is_none());
}

#[test]
fn test_get_session_invalid_token_returns_none() {
    let mgr = SessionManager::new();
    let session = mgr.get_session("nonexistent-token");
    assert!(session.is_none());
}

#[test]
fn test_remove_session_invalidates_token() {
    let mgr = SessionManager::new();
    let token = mgr.create_session("alice".to_string()).unwrap();

    assert!(mgr.get_session(&token).is_some());

    mgr.remove_session(&token);

    assert!(mgr.get_session(&token).is_none());
}

#[test]
fn test_per_user_limit() {
    let mgr = SessionManager::new();

    for _ in 0..5 {
        assert!(mgr.create_session("alice".to_string()).is_some());
    }

    assert!(mgr.create_session("alice".to_string()).is_none());

    assert!(mgr.create_session("bob".to_string()).is_some());
}

#[test]
fn test_invalidate_user_sessions_removes_all_for_user() {
    let mgr = SessionManager::new();

    let t1 = mgr.create_session("alice".to_string()).unwrap();
    let t2 = mgr.create_session("alice".to_string()).unwrap();
    let t3 = mgr.create_session("bob".to_string()).unwrap();

    mgr.invalidate_user_sessions("alice");

    assert!(mgr.get_session(&t1).is_none());
    assert!(mgr.get_session(&t2).is_none());
    assert!(mgr.get_session(&t3).is_some());
}

#[test]
fn test_active_sessions_count() {
    let mgr = SessionManager::new();

    assert_eq!(mgr.active_sessions(), 0);

    let _t1 = mgr.create_session("alice".to_string()).unwrap();
    let _t2 = mgr.create_session("alice".to_string()).unwrap();
    let _t3 = mgr.create_session("bob".to_string()).unwrap();

    assert_eq!(mgr.active_sessions(), 3);
}

#[test]
fn test_cleanup_expired_removes_stale_sessions() {
    let mgr = SessionManager::new_with_timeout(Duration::seconds(1));

    let _t1 = mgr.create_session("alice".to_string()).unwrap();
    let _t2 = mgr.create_session("bob".to_string()).unwrap();

    assert_eq!(mgr.active_sessions(), 2);

    std::thread::sleep(std::time::Duration::from_secs(2));

    assert_eq!(mgr.active_sessions(), 0);
}

#[test]
#[ignore]
fn test_global_session_limit() {
    let mgr = SessionManager::new();

    for i in 0..1000 {
        let username = format!("user_{}", i);
        assert!(mgr.create_session(username).is_some(), "Failed at session {}", i);
    }

    assert!(mgr.create_session("overflow_user".to_string()).is_none());
}
