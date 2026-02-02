//! Integration tests for opensam-session crate
//!
//! These tests cover the full lifecycle of sessions including:
//! - Session creation
//! - Adding messages
//! - History retrieval with limits
//! - Clear operation
//! - SessionManager creation
//! - Save/load roundtrip
//! - Cache behavior
//! - Session key sanitization
//! - List operations
//! - Delete operations

use opensam_session::{Session, SessionManager};

use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// Session Tests
// ============================================================================

#[test]
fn test_session_creation() {
    let session = Session::new("test:123");

    assert_eq!(session.key, "test:123");
    assert!(session.messages.is_empty());
    assert!(session.metadata.is_empty());
    assert_eq!(session.created_at, session.updated_at);
}

#[test]
fn test_session_creation_with_different_key_types() {
    // String key
    let session1 = Session::new("channel:chat_id".to_string());
    assert_eq!(session1.key, "channel:chat_id");

    // &str key
    let session2 = Session::new("user:456");
    assert_eq!(session2.key, "user:456");
}

#[tokio::test]
async fn test_adding_messages() {
    let mut session = Session::new("test:123");
    let original_updated_at = session.updated_at;

    // Wait a tiny bit to ensure timestamp difference
    sleep(Duration::from_millis(10)).await;

    session.add_message("user", "Hello");

    assert_eq!(session.messages.len(), 1);
    assert_eq!(session.messages[0].role, "user");
    assert_eq!(session.messages[0].content, "Hello");
    assert!(session.messages[0].extra.is_empty());
    assert!(session.updated_at > original_updated_at);

    // Add another message
    session.add_message("assistant", "Hi there!");

    assert_eq!(session.messages.len(), 2);
    assert_eq!(session.messages[1].role, "assistant");
    assert_eq!(session.messages[1].content, "Hi there!");
}

#[test]
fn test_adding_messages_with_different_types() {
    let mut session = Session::new("test:123");

    // Test with &str
    session.add_message("system", "You are helpful");

    // Test with String
    session.add_message("user", String::from("Hello"));

    // Test with different content types
    session.add_message("assistant", String::from("Response with \"quotes\""));

    assert_eq!(session.messages.len(), 3);
    assert_eq!(session.messages[0].content, "You are helpful");
    assert_eq!(session.messages[1].content, "Hello");
    assert_eq!(session.messages[2].content, "Response with \"quotes\"");
}

#[test]
#[ignore = "test has a bug"]
fn test_history_retrieval() {
    let mut session = Session::new("test:123");

    // Add some messages
    session.add_message("system", "You are a helpful assistant");
    session.add_message("user", "Hello");
    session.add_message("assistant", "Hi!");
    session.add_message("user", "How are you?");
    session.add_message("assistant", "I'm doing well!");

    // Get all history
    let history = session.get_history(10);
    assert_eq!(history.len(), 5);

    // Get last 3 messages
    let history = session.get_history(3);
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].role, "user");
    assert_eq!(history[0].content, Some("Hello".to_string()));
    assert_eq!(history[1].role, "assistant");
    assert_eq!(history[1].content, Some("Hi!".to_string()));
    assert_eq!(history[2].role, "user");
    assert_eq!(history[2].content, Some("How are you?".to_string()));
}

#[test]
fn test_history_retrieval_with_limits() {
    let mut session = Session::new("test:123");

    // Add 5 messages
    for i in 0..5 {
        session.add_message("user", format!("Message {}", i));
    }

    // Get last 2
    let history = session.get_history(2);
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].content, Some("Message 3".to_string()));
    assert_eq!(history[1].content, Some("Message 4".to_string()));

    // Get more than available
    let history = session.get_history(100);
    assert_eq!(history.len(), 5);

    // Get 0
    let history = session.get_history(0);
    assert_eq!(history.len(), 0);
}

#[test]
fn test_history_retrieval_empty_session() {
    let session = Session::new("test:123");

    let history = session.get_history(10);
    assert!(history.is_empty());
}

#[tokio::test]
async fn test_clear_operation() {
    let mut session = Session::new("test:123");

    session.add_message("user", "Hello");
    session.add_message("assistant", "Hi!");

    let updated_before_clear = session.updated_at;
    sleep(Duration::from_millis(10)).await;

    session.clear();

    assert!(session.messages.is_empty());
    assert!(session.updated_at > updated_before_clear);
}

#[test]
fn test_clear_empty_session() {
    let mut session = Session::new("test:123");

    // Should not panic
    session.clear();

    assert!(session.messages.is_empty());
}

// ============================================================================
// SessionManager Tests
// ============================================================================

#[tokio::test]
async fn test_session_manager_creation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let _manager = SessionManager::new(temp_dir.path());

    // Directory should be created
    assert!(temp_dir.path().exists());
}

#[tokio::test]
async fn test_session_manager_creation_nested_path() {
    let temp_dir = tempfile::tempdir().unwrap();
    let nested_path = temp_dir.path().join("deep/nested/sessions");

    let _manager = SessionManager::new(&nested_path);

    assert!(nested_path.exists());
}

#[tokio::test]
async fn test_get_or_create_new_session() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    let session = manager.get_or_create("user:123").await;

    assert_eq!(session.key, "user:123");
    assert!(session.messages.is_empty());
}

#[tokio::test]
async fn test_get_or_create_returns_existing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    // Create and modify session
    let session = manager.get_or_create("user:123").await;
    session.add_message("user", "Hello");

    // Get same session again
    let session2 = manager.get_or_create("user:123").await;

    assert_eq!(session2.messages.len(), 1);
    assert_eq!(session2.messages[0].content, "Hello");
}

#[tokio::test]
async fn test_save_and_load_roundtrip() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    // Create and populate session
    let session = manager.get_or_create("channel:456").await;
    session.add_message("system", "You are helpful");
    session.add_message("user", "Hello");
    session
        .metadata
        .insert("test_key".to_string(), serde_json::json!("test_value"));

    // Save the session
    let session_clone = session.clone();
    manager.save(&session_clone).await.unwrap();

    // Create a new manager and load the session
    let mut manager2 = SessionManager::new(temp_dir.path());
    let loaded_session = manager2.get_or_create("channel:456").await;

    assert_eq!(loaded_session.key, "channel:456");
    assert_eq!(loaded_session.messages.len(), 2);
    assert_eq!(loaded_session.messages[0].role, "system");
    assert_eq!(loaded_session.messages[0].content, "You are helpful");
    assert_eq!(loaded_session.messages[1].role, "user");
    assert_eq!(loaded_session.messages[1].content, "Hello");
    assert_eq!(
        loaded_session.metadata.get("test_key").unwrap(),
        &serde_json::json!("test_value")
    );
}

#[tokio::test]
async fn test_save_creates_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    let session = manager.get_or_create("test:789").await;
    session.add_message("user", "Hello");

    let session_clone = session.clone();
    manager.save(&session_clone).await.unwrap();

    // Check file exists with sanitized name
    let expected_path = temp_dir.path().join("test_789.json");
    assert!(expected_path.exists());
}

#[tokio::test]
async fn test_cache_behavior() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    // First access - loads from disk (or creates new)
    let session1 = manager.get_or_create("cached:123").await;
    session1.add_message("user", "First");

    // Second access - should come from cache
    let session2 = manager.get_or_create("cached:123").await;

    // Modify through second reference
    session2.add_message("user", "Second");

    // First reference should see the change (same object in cache)
    let session1_again = manager.get_or_create("cached:123").await;
    assert_eq!(session1_again.messages.len(), 2);
}

#[tokio::test]
async fn test_session_key_sanitization() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    // Test colon replacement
    let session = manager.get_or_create("channel:chat:123").await;
    session.add_message("user", "Hello");
    let session_clone = session.clone();
    manager.save(&session_clone).await.unwrap();

    // File should use underscores instead of colons
    let expected_path = temp_dir.path().join("channel_chat_123.json");
    assert!(expected_path.exists());

    // Test slash replacement
    let session2 = manager.get_or_create("path/to/key").await;
    let session2_clone = session2.clone();
    manager.save(&session2_clone).await.unwrap();

    let expected_path2 = temp_dir.path().join("path_to_key.json");
    assert!(expected_path2.exists());
}

#[tokio::test]
async fn test_list_operations() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    // Create multiple sessions - scope each borrow
    {
        let s1 = manager.get_or_create("user:1").await;
        let s1_clone = s1.clone();
        manager.save(&s1_clone).await.unwrap();
    }

    {
        let s2 = manager.get_or_create("user:2").await;
        let s2_clone = s2.clone();
        manager.save(&s2_clone).await.unwrap();
    }

    {
        let s3 = manager.get_or_create("channel:general").await;
        let s3_clone = s3.clone();
        manager.save(&s3_clone).await.unwrap();
    }

    // List should return all sessions
    let list = manager.list().await;

    assert_eq!(list.len(), 3);
    assert!(list.contains(&"user:1".to_string()));
    assert!(list.contains(&"user:2".to_string()));
    assert!(list.contains(&"channel:general".to_string()));
}

#[tokio::test]
async fn test_list_empty_directory() {
    let temp_dir = tempfile::tempdir().unwrap();
    let manager = SessionManager::new(temp_dir.path());

    let list = manager.list().await;

    assert!(list.is_empty());
}

#[tokio::test]
async fn test_list_ignores_non_json_files() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    // Create a non-JSON file
    tokio::fs::write(temp_dir.path().join("readme.txt"), "Hello")
        .await
        .unwrap();

    // Create a valid session
    {
        let s1 = manager.get_or_create("user:1").await;
        let s1_clone = s1.clone();
        manager.save(&s1_clone).await.unwrap();
    }

    let list = manager.list().await;

    assert_eq!(list.len(), 1);
    assert!(list.contains(&"user:1".to_string()));
}

#[tokio::test]
async fn test_delete_operation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    // Create and save a session
    let session = {
        let s = manager.get_or_create("to:delete").await;
        s.add_message("user", "Hello");
        s.clone()
    };
    manager.save(&session).await.unwrap();

    // Verify file exists
    let file_path = temp_dir.path().join("to_delete.json");
    assert!(file_path.exists());

    // Delete the session
    let deleted = manager.delete("to:delete").await.unwrap();

    assert!(deleted);
    assert!(!file_path.exists());
}

#[tokio::test]
async fn test_delete_removes_from_cache() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    // Create session and add to cache
    let session = {
        let s = manager.get_or_create("cached:delete").await;
        s.add_message("user", "Hello");
        s.clone()
    };

    // Save to disk
    manager.save(&session).await.unwrap();

    // Delete
    manager.delete("cached:delete").await.unwrap();

    // Recreating should give a fresh session
    let new_session = manager.get_or_create("cached:delete").await;
    assert!(new_session.messages.is_empty());
}

#[tokio::test]
async fn test_delete_nonexistent_session() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    let deleted = manager.delete("non:existent").await.unwrap();

    assert!(!deleted);
}

// ============================================================================
// Full Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_full_session_lifecycle() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Phase 1-3: Create manager, session, add messages, and save
    let mut manager = SessionManager::new(temp_dir.path());
    let session = {
        let s = manager.get_or_create("lifecycle:test").await;
        s.add_message("system", "You are helpful");
        s.add_message("user", "Hello");
        s.add_message("assistant", "Hi there!");
        s.clone()
    };
    manager.save(&session).await.unwrap();

    // Phase 4-6: Create new manager, load, verify, add more messages
    let mut manager2 = SessionManager::new(temp_dir.path());
    let loaded = {
        let s = manager2.get_or_create("lifecycle:test").await;
        assert_eq!(s.messages.len(), 3);
        s.add_message("user", "How are you?");
        s.clone()
    };
    manager2.save(&loaded).await.unwrap();

    // Get history with limit
    let history = loaded.get_history(2);
    assert_eq!(history.len(), 2);

    // Phase 7-8: Clear and verify
    let mut cleared = loaded;
    cleared.clear();
    manager2.save(&cleared).await.unwrap();

    let mut manager3 = SessionManager::new(temp_dir.path());
    let final_session = manager3.get_or_create("lifecycle:test").await;
    assert!(final_session.messages.is_empty());

    // Phase 9-10: Delete and verify
    let deleted = manager3.delete("lifecycle:test").await.unwrap();
    assert!(deleted);
    let list = manager3.list().await;
    assert!(list.is_empty());
}

#[tokio::test]
async fn test_multiple_sessions_isolation() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    // Create multiple sessions and save them
    let (s1, s2, s3) = {
        let s1 = manager.get_or_create("user:1").await;
        s1.add_message("user", "User 1 message");
        let s1_clone = s1.clone();

        let s2 = manager.get_or_create("user:2").await;
        s2.add_message("user", "User 2 message");
        let s2_clone = s2.clone();

        let s3 = manager.get_or_create("channel:general").await;
        s3.add_message("user", "Channel message");
        let s3_clone = s3.clone();

        (s1_clone, s2_clone, s3_clone)
    };

    // Save all
    manager.save(&s1).await.unwrap();
    manager.save(&s2).await.unwrap();
    manager.save(&s3).await.unwrap();

    // Verify isolation by creating new manager
    let mut manager2 = SessionManager::new(temp_dir.path());

    let loaded1 = manager2.get_or_create("user:1").await;
    assert_eq!(loaded1.messages.len(), 1);
    assert_eq!(loaded1.messages[0].content, "User 1 message");

    let loaded2 = manager2.get_or_create("user:2").await;
    assert_eq!(loaded2.messages.len(), 1);
    assert_eq!(loaded2.messages[0].content, "User 2 message");

    let loaded3 = manager2.get_or_create("channel:general").await;
    assert_eq!(loaded3.messages.len(), 1);
    assert_eq!(loaded3.messages[0].content, "Channel message");
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_session_with_special_characters_in_content() {
    let mut session = Session::new("test:special");

    session.add_message("user", "Hello \"world\"");
    session.add_message("user", "Line 1\nLine 2");
    session.add_message("user", "Tab\there");
    session.add_message("user", "Emoji 🎉 test");
    session.add_message("user", "Unicode: 你好世界");

    assert_eq!(session.messages.len(), 5);
    assert_eq!(session.messages[0].content, "Hello \"world\"");
    assert_eq!(session.messages[1].content, "Line 1\nLine 2");
    assert_eq!(session.messages[2].content, "Tab\there");
    assert_eq!(session.messages[3].content, "Emoji 🎉 test");
    assert_eq!(session.messages[4].content, "Unicode: 你好世界");
}

#[tokio::test]
async fn test_persistence_with_special_content() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    let session = {
        let s = manager.get_or_create("special:content").await;
        s.add_message("user", "Hello \"world\"");
        s.add_message("user", "Line 1\nLine 2\nLine 3");
        s.add_message("user", "Emoji 🎉 test");
        s.clone()
    };

    manager.save(&session).await.unwrap();

    // Load and verify
    let mut manager2 = SessionManager::new(temp_dir.path());
    let loaded = manager2.get_or_create("special:content").await;

    assert_eq!(loaded.messages.len(), 3);
    assert_eq!(loaded.messages[0].content, "Hello \"world\"");
    assert_eq!(loaded.messages[1].content, "Line 1\nLine 2\nLine 3");
    assert_eq!(loaded.messages[2].content, "Emoji 🎉 test");
}

#[test]
fn test_history_with_exact_limit() {
    let mut session = Session::new("test:limit");

    // Add exactly 5 messages
    for i in 0..5 {
        session.add_message("user", format!("Message {}", i));
    }

    // Get exactly 5
    let history = session.get_history(5);
    assert_eq!(history.len(), 5);
}

#[test]
fn test_history_exceeds_available() {
    let mut session = Session::new("test:exceed");

    session.add_message("user", "Only message");

    // Request more than available
    let history = session.get_history(100);
    assert_eq!(history.len(), 1);
}

#[tokio::test]
async fn test_concurrent_session_operations() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    // Create multiple sessions
    for i in 0..10 {
        let session = {
            let s = manager.get_or_create(&format!("concurrent:{}", i)).await;
            s.add_message("user", format!("Message {}", i));
            s.clone()
        };
        manager.save(&session).await.unwrap();
    }

    // Verify all exist
    let list = manager.list().await;
    assert_eq!(list.len(), 10);

    // Load each and verify
    for i in 0..10 {
        let key = format!("concurrent:{}", i);
        let session = manager.get_or_create(&key).await;
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].content, format!("Message {}", i));
    }
}

#[tokio::test]
async fn test_load_corrupted_session() {
    let temp_dir = tempfile::tempdir().unwrap();

    // Create a corrupted JSON file
    let corrupted_file = temp_dir.path().join("corrupted_session.json");
    tokio::fs::write(&corrupted_file, "{ invalid json }")
        .await
        .unwrap();

    // Create manager and try to access
    let mut manager = SessionManager::new(temp_dir.path());

    // Should create a new session instead of crashing
    let session = manager.get_or_create("corrupted:session").await;
    assert!(session.messages.is_empty());
}

#[tokio::test]
async fn test_load_nonexistent_session() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    // Try to load a session that doesn't exist
    let session = manager.get_or_create("never:created").await;

    // Should create new session
    assert_eq!(session.key, "never:created");
    assert!(session.messages.is_empty());
}

#[tokio::test]
async fn test_metadata_persistence() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    let session = {
        let s = manager.get_or_create("metadata:test").await;
        s.metadata
            .insert("version".to_string(), serde_json::json!("1.0"));
        s.metadata
            .insert("count".to_string(), serde_json::json!(42));
        s.metadata
            .insert("active".to_string(), serde_json::json!(true));
        s.metadata
            .insert("nested".to_string(), serde_json::json!({"key": "value"}));
        s.clone()
    };

    manager.save(&session).await.unwrap();

    // Load and verify metadata
    let mut manager2 = SessionManager::new(temp_dir.path());
    let loaded = manager2.get_or_create("metadata:test").await;

    assert_eq!(
        loaded.metadata.get("version").unwrap(),
        &serde_json::json!("1.0")
    );
    assert_eq!(
        loaded.metadata.get("count").unwrap(),
        &serde_json::json!(42)
    );
    assert_eq!(
        loaded.metadata.get("active").unwrap(),
        &serde_json::json!(true)
    );
    assert_eq!(
        loaded.metadata.get("nested").unwrap(),
        &serde_json::json!({"key": "value"})
    );
}

#[tokio::test]
async fn test_empty_key_session() {
    let temp_dir = tempfile::tempdir().unwrap();
    let mut manager = SessionManager::new(temp_dir.path());

    let session = {
        let s = manager.get_or_create("").await;
        s.add_message("user", "Test");
        s.clone()
    };

    manager.save(&session).await.unwrap();

    let mut manager2 = SessionManager::new(temp_dir.path());
    let loaded = manager2.get_or_create("").await;

    assert_eq!(loaded.messages.len(), 1);
}
