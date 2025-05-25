//! Simple test to verify nextest setup

#[test]
fn test_basic_math() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn test_string_operations() {
    let hello = "Hello";
    let world = "World";
    let result = format!("{} {}", hello, world);
    assert_eq!(result, "Hello World");
}

#[tokio::test]
async fn test_async_sleep() {
    use tokio::time::{sleep, Duration};
    
    let start = std::time::Instant::now();
    sleep(Duration::from_millis(100)).await;
    let elapsed = start.elapsed();
    
    assert!(elapsed >= Duration::from_millis(100));
    assert!(elapsed < Duration::from_millis(200));
}