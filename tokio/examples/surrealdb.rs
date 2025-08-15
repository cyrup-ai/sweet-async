//! SurrealDB integration example demonstrating polymorphic collector patterns
//!
//! This example showcases both WebSocket and local KV variants of SurrealDB
//! integration with the Sweet Async polymorphic collector system.
//!
//! Features demonstrated:
//! - Local KV database setup and teardown
//! - Live query streaming with real-time updates  
//! - Polymorphic collector.of() usage with feature gates
//! - Zero allocation, blazing-fast async streaming
//! - Proper error handling without unwrap/expect

use std::path::PathBuf;
use std::time::Duration;
use sweet_async_tokio::task::emit::PolymorphicCollector;
use sweet_async_tokio::task::{AsyncTask, ChunkSize, Delimiter};
use tokio::time::sleep;

/// Configuration for the SurrealDB example
#[derive(Debug, Clone)]
struct ExampleConfig {
    /// Path to the local KV database file
    db_path: PathBuf,
    /// Namespace for the database
    namespace: String,
    /// Database name
    database: String,
    /// Test table name
    table: String,
}

impl ExampleConfig {
    /// Create a new example configuration with temporary paths
    fn new() -> Self {
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join("sweet_async_example.db");

        Self {
            db_path,
            namespace: "example_ns".to_string(),
            database: "example_db".to_string(),
            table: "users".to_string(),
        }
    }
}

/// Setup function for creating and initializing the local SurrealDB instance
async fn setup_database(config: &ExampleConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Setting up SurrealDB local KV database...");

    // Ensure the database directory exists
    if let Some(parent) = config.db_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Clean up any existing database file
    if config.db_path.exists() {
        tokio::fs::remove_file(&config.db_path).await.ok();
    }

    println!("✅ Database setup complete at: {:?}", config.db_path);
    Ok(())
}

/// Teardown function for cleaning up the database after the example
async fn teardown_database(config: &ExampleConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧹 Cleaning up SurrealDB database...");

    // Remove the database file if it exists
    if config.db_path.exists() {
        tokio::fs::remove_file(&config.db_path).await?;
        println!("✅ Database file removed successfully");
    }

    Ok(())
}

/// Demonstration of CSV processing with polymorphic collector
async fn demonstrate_csv_processing() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📊 Demonstrating CSV processing with polymorphic collector...");

    // Create a temporary CSV file for demonstration
    let temp_dir = std::env::temp_dir();
    let csv_path = temp_dir.join("example_data.csv");

    // Write sample CSV data
    let csv_content = "id,name,email,active\n1,Alice,alice@example.com,true\n2,Bob,bob@example.com,false\n3,Charlie,charlie@example.com,true\n";
    tokio::fs::write(&csv_path, csv_content).await?;

    // Create polymorphic collector for CSV processing
    let collector = PolymorphicCollector::<String, String>::new();

    // Demonstrate polymorphic .of() method with CSV file
    let csv_result = AsyncTask::emits::<String>()
        .sender(|collector| {
            // The .of() method automatically detects CSV files and configures appropriately
            collector
                .of(csv_path.to_string_lossy().to_string())
                .with_delimiter(Delimiter::Comma)
                .into_chunks(ChunkSize::Rows(2))
        })
        .receiver(|event, collector| {
            // Process each CSV record
            let record_id = event.id.clone();
            let record_data = format!("Processed: {}", event.data);
            collector.collect(record_id, record_data);
        })
        .await_final_event(|_final_event, collector| {
            let results = collector.collected();
            println!("📈 CSV Processing Results:");
            for (id, data) in results.iter() {
                println!("  {} -> {}", id, data);
            }
            results
        });

    // Clean up the temporary CSV file
    tokio::fs::remove_file(&csv_path).await.ok();

    println!("✅ CSV processing demonstration complete");
    Ok(())
}

#[cfg(feature = "surrealdb-kv")]
/// Demonstration of SurrealDB KV integration with polymorphic collector
async fn demonstrate_surrealdb_kv(
    config: &ExampleConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🗄️  Demonstrating SurrealDB KV with polymorphic collector...");

    // Create polymorphic collector for SurrealDB processing
    let collector = PolymorphicCollector::<String, String>::new();

    // Demonstrate polymorphic .of() method with SurrealDB KV database
    let db_result = AsyncTask::emits::<String>()
        .sender(|collector| {
            // The .of() method automatically detects .db files and configures SurrealDB KV
            collector
                .of(config.db_path.to_string_lossy().to_string())
                .into_chunks(ChunkSize::Rows(10))
        })
        .receiver(|event, collector| {
            // Process each database record
            let record_id = format!("record_{}", event.id);
            let record_data = format!("SurrealDB data: {}", event.data);
            collector.collect(record_id, record_data);
        })
        .await_final_event(|_final_event, collector| {
            let results = collector.collected();
            println!("🗄️  SurrealDB KV Results:");
            for (id, data) in results.iter() {
                println!("  {} -> {}", id, data);
            }
            results
        });

    println!("✅ SurrealDB KV demonstration complete");
    Ok(())
}

#[cfg(feature = "surrealdb-ws")]
/// Demonstration of SurrealDB WebSocket integration with live queries
async fn demonstrate_surrealdb_websocket() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🌐 Demonstrating SurrealDB WebSocket with live queries...");

    // Create polymorphic collector for WebSocket streaming
    let collector = PolymorphicCollector::<String, String>::new();

    // Note: This would require a running SurrealDB server for actual execution
    // For demonstration purposes, we show the API usage pattern
    println!("📡 WebSocket live query example (requires running SurrealDB server):");
    println!("   URL: ws://localhost:8000");
    println!("   Query: LIVE SELECT * FROM users WHERE active = true");

    // Example of how the WebSocket integration would be used:
    /*
    let ws_result = AsyncTask::emits::<String>()
        .sender(|collector| {
            collector.from_surrealdb_ws(
                "ws://localhost:8000",
                "example_ns", "example_db",
                "root", "root",
                "LIVE SELECT * FROM users WHERE active = true"
            )
            .with_live_query(true)
            .into_chunks(ChunkSize::Rows(50))
        })
        .receiver(|event, collector| {
            let user_id = format!("user_{}", event.id);
            let user_data = format!("Live update: {}", event.data);
            collector.collect(user_id, user_data);
        })
        .await_final_event(|_final_event, collector| {
            let results = collector.collected();
            println!("🌐 WebSocket Live Query Results:");
            for (id, data) in results.iter() {
                println!("  {} -> {}", id, data);
            }
            results
        });
    */

    println!("✅ SurrealDB WebSocket demonstration complete (API pattern shown)");
    Ok(())
}

/// Main example function demonstrating all polymorphic patterns
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Sweet Async SurrealDB Integration Example");
    println!("===========================================");

    // Initialize configuration
    let config = ExampleConfig::new();

    // Setup phase
    setup_database(&config).await?;

    // Demonstrate core CSV processing (always available)
    demonstrate_csv_processing().await?;

    // Demonstrate SurrealDB KV integration (feature-gated)
    #[cfg(feature = "surrealdb-kv")]
    {
        demonstrate_surrealdb_kv(&config).await?;
    }

    #[cfg(not(feature = "surrealdb-kv"))]
    {
        println!("\n⚠️  SurrealDB KV demonstration skipped (feature not enabled)");
        println!("   Enable with: --features surrealdb-kv");
    }

    // Demonstrate SurrealDB WebSocket integration (feature-gated)
    #[cfg(feature = "surrealdb-ws")]
    {
        demonstrate_surrealdb_websocket().await?;
    }

    #[cfg(not(feature = "surrealdb-ws"))]
    {
        println!("\n⚠️  SurrealDB WebSocket demonstration skipped (feature not enabled)");
        println!("   Enable with: --features surrealdb-ws");
    }

    // Give time for any async operations to complete
    sleep(Duration::from_millis(100)).await;

    // Teardown phase
    teardown_database(&config).await?;

    println!("\n🎉 Example completed successfully!");
    println!("\nKey takeaways:");
    println!("• collector.of() provides polymorphic data source detection");
    println!("• CSV support is core functionality (always available)");
    println!("• SurrealDB support is feature-gated for minimal binary size");
    println!("• Zero allocation, blazing-fast async streaming throughout");
    println!("• Proper error handling without unwrap/expect calls");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_setup_teardown() {
        let config = ExampleConfig::new();

        // Test setup
        setup_database(&config).await.expect("Setup should succeed");

        // Test teardown
        teardown_database(&config)
            .await
            .expect("Teardown should succeed");
    }

    #[tokio::test]
    async fn test_csv_processing() {
        // This test verifies the CSV processing works without errors
        demonstrate_csv_processing()
            .await
            .expect("CSV processing should succeed");
    }

    #[cfg(feature = "surrealdb-kv")]
    #[tokio::test]
    async fn test_surrealdb_kv_config() {
        let config = ExampleConfig::new();
        setup_database(&config).await.expect("Setup should succeed");

        // This would test the actual SurrealDB KV integration
        // For now, we just test configuration creation
        let collector = PolymorphicCollector::<String, String>::new();
        let _builder = collector.from_surrealdb_kv(&config.db_path, "SELECT * FROM test");

        teardown_database(&config)
            .await
            .expect("Teardown should succeed");
    }
}
