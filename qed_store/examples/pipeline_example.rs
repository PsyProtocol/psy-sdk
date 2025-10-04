use qed_store::queue::ResilientRedisConnection;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚀 Testing ResilientRedisConnection Pipeline");
    
    let redis = ResilientRedisConnection::new("redis://127.0.0.1:6379").await?;
    println!("✅ Connected to Redis");
    
    if !redis.health_check().await {
        println!("❌ Redis health check failed");
        return Ok(());
    }
    println!("✅ Health check passed");
    
    // 测试数据
    let test_count = 50;
    let test_data: Vec<_> = (0..test_count)
        .map(|i| (format!("pipeline_test_{}", i), format!("value_{}", i)))
        .collect();
    
    // 测试1：单个操作
    println!("\n📊 Testing {} single operations...", test_count);
    let start = Instant::now();
    for (key, value) in &test_data {
        redis.set(key.clone(), value.clone()).await?;
    }
    let single_duration = start.elapsed();
    println!("⏱️  Single operations took: {:?}", single_duration);
    
    // 清理
    for (key, _) in &test_data {
        redis.del(key.clone()).await.ok();
    }
    
    // 测试2：Pipeline操作
    println!("\n📊 Testing {} pipeline operations...", test_count);
    let start = Instant::now();
    let mut builder = redis.cmd_builder();
    for (key, value) in &test_data {
        builder = builder.set(key.clone(), value.clone());
    }
    builder.execute(&redis).await?;
    let pipeline_duration = start.elapsed();
    println!("⏱️  Pipeline operations took: {:?}", pipeline_duration);
    
    // 性能对比
    let speedup = single_duration.as_secs_f64() / pipeline_duration.as_secs_f64();
    println!("\n🚀 Pipeline speedup: {:.2}x faster!", speedup);
    
    // 验证数据
    println!("\n🔍 Verifying data...");
    let mut verified = 0;
    for (key, expected_value) in &test_data {
        let actual: String = redis.get(key.clone()).await?;
        if actual == *expected_value {
            verified += 1;
        }
    }
    println!("✅ Verified {}/{} keys", verified, test_count);
    
    // 测试3：混合命令Pipeline
    println!("\n📊 Testing mixed commands pipeline...");
    let start = Instant::now();
    redis.cmd_builder()
        .set("string_key", "string_value")
        .hset("hash_key", "field1", "value1")
        .hset("hash_key", "field2", "value2")
        .sadd("set_key", "member1")
        .sadd("set_key", "member2")
        .execute(&redis).await?;
    let mixed_duration = start.elapsed();
    println!("⏱️  Mixed commands took: {:?}", mixed_duration);
    
    // 验证混合命令结果
    let string_val: String = redis.get("string_key").await?;
    let hash_val: String = redis.hget("hash_key", "field1").await?;
    let set_members: Vec<String> = redis.smembers("set_key").await?;
    
    println!("📋 Mixed commands results:");
    println!("   String: {}", string_val);
    println!("   Hash field: {}", hash_val);
    println!("   Set members: {:?}", set_members);
    
    // 最终清理
    println!("\n🧹 Cleaning up...");
    let mut cleanup = redis.cmd_builder();
    for (key, _) in &test_data {
        cleanup = cleanup.del(key.clone());
    }
    cleanup = cleanup
        .del("string_key")
        .del("hash_key")
        .del("set_key");
    cleanup.execute(&redis).await?;
    
    // 连接统计
    let stats = redis.get_stats().await;
    println!("\n📊 Connection Stats:");
    println!("   Connected: {}", stats.connected);
    
    println!("\n✅ All tests completed successfully!");
    Ok(())
}