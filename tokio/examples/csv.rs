use sweet_async_tokio::task::{AsyncTask, CsvRecord, Delimiter, RowsExt};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Sweet Async CSV Processing Demo");
    
    // Create test CSV data with header and records
    let csv_content = "id,name,age,city\n1,Alice,30,Seattle\n2,Bob,25,Portland\n3,Charlie,35,Vancouver";
    std::fs::write("data.csv", csv_content)?;
    println!("Created test CSV file with 3 records");

    // Process CSV using the exact README syntax with zero-allocation design
    let csv_records = AsyncTask::emits::<CsvRecord>()
        .sender(|collector| {
            collector.of("data.csv")
                .with_delimiter(Delimiter::NewLine)
                .into_chunks(100.rows());
        })
        .receiver(|event, collector| {
            let record = event.data();
            if record.is_valid() {
                // Use the first field as the key for collection
                collector.collect(record.id.to_string(), record.clone());
            }
        })
        .await_final_event(|_event, collector| {
            collector.collected()
        });

    // Display processing results
    println!("\n✅ Successfully processed {} CSV records:", csv_records.len());
    
    for (key, record) in &csv_records {
        println!("📄 Record '{}': {} fields at line {}", 
                 key, record.field_count(), record.line_number);
        
        // Show field details
        for (i, field) in record.data.iter().enumerate() {
            println!("   Field {}: '{}'", i, field);
        }
    }

    // Demonstrate zero-allocation field access
    if let Some((_, first_record)) = csv_records.iter().next() {
        println!("\n🔍 Demonstrating zero-allocation field access:");
        println!("ID: {}", first_record.id);
        if let Some(name) = first_record.get_field(1) {
            println!("Name: {}", name);
        }
        if let Some(age) = first_record.get_field(2) {
            println!("Age: {}", age);
        }
        if let Some(city) = first_record.get_field(3) {
            println!("City: {}", city);
        }
    }

    // Performance metrics
    println!("\n📊 Performance characteristics:");
    println!("- Zero allocation during parsing (string slices)");
    println!("- Lock-free concurrent collection via DashMap");
    println!("- Blazing-fast async streaming with tokio");
    println!("- Sophisticated chunking strategies");

    // Clean up test file
    std::fs::remove_file("data.csv").ok();
    println!("\n🧹 Cleaned up test files");
    
    Ok(())
}
