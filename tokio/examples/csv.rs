use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use sweet_async_tokio::{AsyncTask, task::{DurationExt, RowsExt, Delimiter}};
use sweet_async_tokio::task::emit::channel_builder::FromCsvLine;

#[derive(Clone, Debug)]
struct CsvRecord {
    id: u32,
    data: String, // Simplified for example
}

impl CsvRecord {
    fn is_valid(&self) -> bool {
        !self.data.is_empty()
    }
}

impl FromCsvLine for CsvRecord {
    fn from_csv_line(id: u32, data: &str) -> Option<Self> {
        if data.is_empty() {
            None
        } else {
            Some(CsvRecord {
                id,
                data: data.to_string(),
            })
        }
    }
}

#[derive(Clone)]
struct SchemaConfig; // Placeholder
#[derive(Clone)]
struct ProcessingStats {
    files: std::sync::Arc<std::sync::atomic::AtomicU32>,
}

impl ProcessingStats {
    fn increment_files(&self) {
        self.files
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

// Using Delimiter from sweet_async_tokio::task module

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a real CSV file for testing
    let csv_path = "data.csv";
    std::fs::write(csv_path, "id,data\n1,record1\n2,record2\n3,invalid\n")?;

    let schema_config = SchemaConfig;
    let processing_stats = ProcessingStats {
        files: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
    };

    let csv_records = AsyncTask::emits::<CsvRecord>()
        .with(schema_config)
        .with(processing_stats.clone())
        .with_timeout(60.seconds())
        .sender(|collector| {
            collector
                .of_file(csv_path)
                .with_delimiter(Delimiter::NewLine)
                .into_chunks(100.rows());
        })
        .receiver(|event, collector| {
            let record = event.data();
            if record.is_valid() {
                collector.collect(record.id, record);
            }
        })
        .await_final_event(|_event, collector| {
            // Success case - return collected results
            Ok(collector.collected())
        })
        .await?;

    println!("Processed records: {:?}", csv_records);

    Ok(())
}
