use qed_store::queue::{new_resilient_redis_connection, ResilientRedisConnection};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize basic logging (tracing_subscriber not available in example)
    // tracing_subscriber::init();
    
    // Create resilient connection
    let redis_conn = new_resilient_redis_connection("redis://127.0.0.1:6379").await?;
    
    println!("✅ ResilientRedisConnection created successfully");
    
    // Test health check
    if redis_conn.health_check().await {
        println!("✅ Redis health check passed");
    } else {
        println!("❌ Redis health check failed");
        return Ok(());
    }
    
    // Test basic operations
    redis_conn.set("test_key".to_string(), "test_value".to_string()).await?;
    println!("✅ SET operation successful");
    
    let value: String = redis_conn.get("test_key".to_string()).await?;
    println!("✅ GET operation successful: {}", value);
    
    // Test BLPOP (non-blocking with timeout)
    println!("Testing BLPOP with 2 second timeout...");
    match redis_conn.blpop("test_queue".to_string(), 2).await? {
        Some((key, data)) => {
            println!("✅ BLPOP got data from {}: {:?}", key, data);
        }
        None => {
            println!("⏰ BLPOP timed out (expected)");
        }
    }
    
    // Test connection stats
    let stats = redis_conn.get_stats().await;
    println!("📊 Connection stats: {:?}", stats);
    
    // Test resilience by doing many operations
    println!("Testing resilience with 100 operations...");
    for i in 0..100 {
        let key = format!("test_key_{}", i);
        let value = format!("test_value_{}", i);
        
        redis_conn.set(key.clone(), value.clone()).await?;
        let retrieved: String = redis_conn.get(key).await?;
        assert_eq!(value, retrieved);
        
        if i % 10 == 0 {
            println!("✅ Completed {} operations", i + 1);
        }
    }
    
    println!("✅ All operations completed successfully!");
    
    // Final stats
    let final_stats = redis_conn.get_stats().await;
    println!("📊 Final connection stats:");
    println!("   Connected: {}", final_stats.connected);
    
    Ok(())
}