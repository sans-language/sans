use serde_json::{json, Value};

fn main() {
    let mut last_val = 0i64;
    for _ in 0..100 {
        let mut obj = serde_json::Map::new();
        for i in 0..1000 {
            obj.insert(format!("key{}", i), json!(i));
        }
        let serialized = serde_json::to_string(&obj).unwrap();
        let parsed: Value = serde_json::from_str(&serialized).unwrap();
        last_val = parsed["key999"].as_i64().unwrap();
    }
    println!("{}", last_val);
}
