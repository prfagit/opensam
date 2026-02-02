//! Integration tests for opensam-bus metadata serialization
//!
//! Tests cover:
//! - Various metadata types (primitives, collections, nested structs)
//! - JSON serialization roundtrips
//! - Edge cases in metadata handling

use opensam_bus::{InboundMessage, OutboundMessage};
use serde::Serialize;
use serde_json::json;

// ============================================================================
// Primitive Type Tests
// ============================================================================

#[test]
fn test_metadata_string() {
    let msg =
        InboundMessage::new("ch", "sender", "chat", "test").with_metadata("key", "string value");

    assert_eq!(msg.metadata.get("key").unwrap(), &json!("string value"));
}

#[test]
fn test_metadata_integer() {
    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("i8", 127i8)
        .with_metadata("i16", 1000i16)
        .with_metadata("i32", 100000i32)
        .with_metadata("i64", 10000000000i64)
        .with_metadata("u8", 255u8)
        .with_metadata("u16", 50000u16)
        .with_metadata("u32", 1000000000u32)
        .with_metadata("u64", 10000000000u64)
        .with_metadata("usize", 100usize);

    assert_eq!(msg.metadata.get("i8").unwrap(), &json!(127));
    assert_eq!(msg.metadata.get("i32").unwrap(), &json!(100000));
    assert_eq!(msg.metadata.get("u64").unwrap(), &json!(10000000000u64));
}

#[test]
fn test_metadata_float() {
    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("f32", std::f32::consts::PI)
        .with_metadata("f64", std::f64::consts::E);

    assert!(msg.metadata.get("f32").unwrap().is_f64());
    assert!(msg.metadata.get("f64").unwrap().is_f64());
}

#[test]
fn test_metadata_bool() {
    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("true_val", true)
        .with_metadata("false_val", false);

    assert_eq!(msg.metadata.get("true_val").unwrap(), &json!(true));
    assert_eq!(msg.metadata.get("false_val").unwrap(), &json!(false));
}

#[test]
fn test_metadata_char() {
    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("char", 'A')
        .with_metadata("emoji", '🚀');

    assert_eq!(msg.metadata.get("char").unwrap(), &json!("A"));
    assert_eq!(msg.metadata.get("emoji").unwrap(), &json!("🚀"));
}

// ============================================================================
// Collection Type Tests
// ============================================================================

#[test]
fn test_metadata_vec() {
    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("numbers", vec![1, 2, 3, 4, 5])
        .with_metadata("strings", vec!["a", "b", "c"]);

    let numbers = msg.metadata.get("numbers").unwrap().as_array().unwrap();
    assert_eq!(numbers.len(), 5);
    assert_eq!(numbers[0], 1);

    let strings = msg.metadata.get("strings").unwrap().as_array().unwrap();
    assert_eq!(strings, &vec![json!("a"), json!("b"), json!("c")]);
}

#[test]
fn test_metadata_array() {
    let arr: [i32; 4] = [10, 20, 30, 40];
    let msg = InboundMessage::new("ch", "sender", "chat", "test").with_metadata("fixed_array", arr);

    let val = msg.metadata.get("fixed_array").unwrap().as_array().unwrap();
    assert_eq!(val.len(), 4);
    assert_eq!(val[2], 30);
}

#[test]
fn test_metadata_tuple() {
    let tuple = ("hello", 42, true);
    let msg = InboundMessage::new("ch", "sender", "chat", "test").with_metadata("tuple", tuple);

    let val = msg.metadata.get("tuple").unwrap().as_array().unwrap();
    assert_eq!(val.len(), 3);
    assert_eq!(val[0], "hello");
    assert_eq!(val[1], 42);
    assert_eq!(val[2], true);
}

#[test]
fn test_metadata_hashmap() {
    use std::collections::HashMap;

    let mut map = HashMap::new();
    map.insert("key1", "value1");
    map.insert("key2", "value2");

    let msg = InboundMessage::new("ch", "sender", "chat", "test").with_metadata("hashmap", map);

    let val = msg.metadata.get("hashmap").unwrap().as_object().unwrap();
    assert_eq!(val.get("key1").unwrap(), &json!("value1"));
    assert_eq!(val.get("key2").unwrap(), &json!("value2"));
}

#[test]
fn test_metadata_option() {
    let some_val: Option<i32> = Some(42);
    let none_val: Option<i32> = None;

    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("some_val", some_val)
        .with_metadata("none_val", none_val);

    assert_eq!(msg.metadata.get("some_val").unwrap(), &json!(42));
    assert!(msg.metadata.get("none_val").unwrap().is_null());
}

// ============================================================================
// Nested Struct Tests
// ============================================================================

#[derive(Serialize)]
struct Location {
    latitude: f64,
    longitude: f64,
    altitude: Option<f64>,
}

#[derive(Serialize)]
struct Agent {
    code_name: String,
    clearance_level: u8,
    location: Location,
    skills: Vec<String>,
}

#[test]
fn test_metadata_nested_struct() {
    let agent = Agent {
        code_name: "007".to_string(),
        clearance_level: 10,
        location: Location {
            latitude: 51.5074,
            longitude: -0.1278,
            altitude: Some(100.0),
        },
        skills: vec!["combat".to_string(), "infiltration".to_string()],
    };

    let msg = InboundMessage::new("ch", "sender", "chat", "test").with_metadata("agent", agent);

    let agent_json = msg.metadata.get("agent").unwrap();
    assert_eq!(agent_json.get("code_name").unwrap(), &json!("007"));
    assert_eq!(agent_json.get("clearance_level").unwrap(), &json!(10));

    let location = agent_json.get("location").unwrap();
    assert_eq!(location.get("latitude").unwrap().as_f64().unwrap(), 51.5074);
    assert_eq!(location.get("altitude").unwrap().as_f64().unwrap(), 100.0);

    let skills = agent_json.get("skills").unwrap().as_array().unwrap();
    assert_eq!(skills.len(), 2);
}

#[derive(Serialize)]
struct Mission {
    id: String,
    objectives: Vec<Objective>,
    priority: Priority,
}

#[derive(Serialize)]
struct Objective {
    name: String,
    completed: bool,
}

#[derive(Serialize)]
#[allow(dead_code)]
enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[test]
fn test_metadata_deeply_nested() {
    let mission = Mission {
        id: "M-2024-001".to_string(),
        objectives: vec![
            Objective {
                name: "Infiltrate".to_string(),
                completed: true,
            },
            Objective {
                name: "Extract".to_string(),
                completed: false,
            },
        ],
        priority: Priority::High,
    };

    let msg = InboundMessage::new("ch", "sender", "chat", "test").with_metadata("mission", mission);

    let mission_json = msg.metadata.get("mission").unwrap();
    assert_eq!(mission_json.get("id").unwrap(), &json!("M-2024-001"));

    let objectives = mission_json.get("objectives").unwrap().as_array().unwrap();
    assert_eq!(objectives.len(), 2);
    assert_eq!(objectives[0].get("name").unwrap(), &json!("Infiltrate"));
    assert_eq!(objectives[0].get("completed").unwrap(), &json!(true));
}

// ============================================================================
// JSON Value Tests
// ============================================================================

#[test]
fn test_metadata_json_value_direct() {
    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("null_val", serde_json::Value::Null)
        .with_metadata("bool_val", json!(true))
        .with_metadata("number_val", json!(42.5))
        .with_metadata("string_val", json!("hello"))
        .with_metadata("array_val", json!([1, 2, 3]))
        .with_metadata("object_val", json!({"key": "value", "num": 10}));

    assert!(msg.metadata.get("null_val").unwrap().is_null());
    assert_eq!(msg.metadata.get("bool_val").unwrap(), &json!(true));
    assert_eq!(msg.metadata.get("number_val").unwrap(), &json!(42.5));
    assert_eq!(msg.metadata.get("string_val").unwrap(), &json!("hello"));

    let arr = msg.metadata.get("array_val").unwrap().as_array().unwrap();
    assert_eq!(arr.len(), 3);

    let obj = msg.metadata.get("object_val").unwrap().as_object().unwrap();
    assert_eq!(obj.get("key").unwrap(), &json!("value"));
}

// ============================================================================
// Serialization Roundtrip Tests
// ============================================================================

#[test]
fn test_metadata_roundtrip_preserves_values() {
    let original = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("string", "value")
        .with_metadata("number", 42)
        .with_metadata("bool", true)
        .with_metadata("array", vec![1, 2, 3]);

    let json_str = serde_json::to_string(&original).expect("Should serialize");
    let deserialized: InboundMessage = serde_json::from_str(&json_str).expect("Should deserialize");

    assert_eq!(deserialized.metadata, original.metadata);
}

#[test]
fn test_outbound_metadata_roundtrip() {
    let original = OutboundMessage::new("ch", "chat", "test");
    // Note: OutboundMessage doesn't have with_metadata builder,
    // but we can test the struct directly
    let mut msg = original.clone();
    msg.metadata.insert("key".to_string(), json!("value"));

    let json_str = serde_json::to_string(&msg).expect("Should serialize");
    let deserialized: OutboundMessage =
        serde_json::from_str(&json_str).expect("Should deserialize");

    assert_eq!(deserialized.metadata.get("key").unwrap(), &json!("value"));
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_metadata_empty_string() {
    let msg = InboundMessage::new("ch", "sender", "chat", "test").with_metadata("empty", "");

    assert_eq!(msg.metadata.get("empty").unwrap(), &json!(""));
}

#[test]
fn test_metadata_special_characters() {
    let special = "Special: \"quoted\" \\ backslash \\n newline \\t tab 🚀";
    let msg = InboundMessage::new("ch", "sender", "chat", "test").with_metadata("special", special);

    // Serialize and deserialize
    let json_str = serde_json::to_string(&msg).unwrap();
    let deserialized: InboundMessage = serde_json::from_str(&json_str).unwrap();

    assert_eq!(
        deserialized
            .metadata
            .get("special")
            .unwrap()
            .as_str()
            .unwrap(),
        special
    );
}

#[test]
fn test_metadata_large_values() {
    let large_string = "x".repeat(10000);
    let large_vec: Vec<i32> = (0..1000).collect();

    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("large_string", &large_string)
        .with_metadata("large_vec", large_vec);

    assert_eq!(
        msg.metadata
            .get("large_string")
            .unwrap()
            .as_str()
            .unwrap()
            .len(),
        10000
    );
    assert_eq!(
        msg.metadata
            .get("large_vec")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1000
    );
}

#[test]
fn test_metadata_many_keys() {
    let mut msg = InboundMessage::new("ch", "sender", "chat", "test");

    for i in 0..100 {
        msg = msg.with_metadata(format!("key_{}", i), i);
    }

    assert_eq!(msg.metadata.len(), 100);

    // Verify serialization handles large metadata
    let json_str = serde_json::to_string(&msg).unwrap();
    let deserialized: InboundMessage = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.metadata.len(), 100);
    assert_eq!(deserialized.metadata.get("key_50").unwrap(), &json!(50));
}

#[test]
fn test_metadata_overwrite_key() {
    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("key", "first")
        .with_metadata("key", "second");

    // Second value should overwrite first
    assert_eq!(msg.metadata.get("key").unwrap(), &json!("second"));
}

// ============================================================================
// Enum Tests
// ============================================================================

#[derive(Serialize)]
#[allow(dead_code)]
enum Status {
    Pending,
    InProgress { progress: f64 },
    Completed { result: String },
}

#[test]
fn test_metadata_enum_unit_variant() {
    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("status", Status::Pending);

    // Unit variants serialize as string
    assert_eq!(msg.metadata.get("status").unwrap(), &json!("Pending"));
}

#[test]
fn test_metadata_enum_struct_variant() {
    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("status", Status::InProgress { progress: 0.75 });

    let status = msg.metadata.get("status").unwrap();
    assert!(status.get("InProgress").is_some());
    assert_eq!(
        status.get("InProgress").unwrap().get("progress").unwrap(),
        &json!(0.75)
    );
}

// ============================================================================
// Combined Tests
// ============================================================================

#[test]
fn test_metadata_all_types_combined() {
    #[derive(Serialize)]
    struct ComplexData {
        name: String,
        count: u32,
        active: bool,
        tags: Vec<String>,
        config: serde_json::Value,
    }

    let complex = ComplexData {
        name: "Test".to_string(),
        count: 42,
        active: true,
        tags: vec!["tag1".to_string(), "tag2".to_string()],
        config: json!({"timeout": 30, "retries": 3}),
    };

    let msg = InboundMessage::new("ch", "sender", "chat", "test")
        .with_metadata("simple", "value")
        .with_metadata("number", 123)
        .with_metadata("float", std::f64::consts::PI)
        .with_metadata("bool", false)
        .with_metadata("array", vec![1, 2, 3])
        .with_metadata("complex", complex);

    // Full roundtrip
    let json_str = serde_json::to_string(&msg).unwrap();
    let deserialized: InboundMessage = serde_json::from_str(&json_str).unwrap();

    assert_eq!(deserialized.metadata.len(), 6);
    assert_eq!(
        deserialized.metadata.get("simple").unwrap(),
        &json!("value")
    );
    assert_eq!(deserialized.metadata.get("number").unwrap(), &json!(123));

    let complex_json = deserialized.metadata.get("complex").unwrap();
    assert_eq!(complex_json.get("name").unwrap(), &json!("Test"));
    assert_eq!(complex_json.get("count").unwrap(), &json!(42));
}
