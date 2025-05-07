#[macro_export]
macro_rules! process_stream {
    // Sender and receiver blocks with result mapping
    ({ $($sender:tt)* }, { |$event:ident| $($receiver:tt)* }, { |$results:ident| $($mapper:tt)* }) => {{
        let task = $crate::emit_stream!({ $($sender)* });
        
        // Collect events
        let $results: Vec<_> = task.map(|$event| {
            $($receiver)*
        }).collect().await;
        
        // Process collected results
        { $($mapper)* }
    }};
    
    // Sender and receiver blocks with simple for_each
    ({ $($sender:tt)* }, { |$event:ident| $($receiver:tt)* }) => {{
        let task = $crate::emit_stream!({ $($sender)* });
        
        // Use task.for_each to consume events
        task.for_each(|$event| async {
            $($receiver)*
        }).await
    }};
    
    // Sender and filter-map-receiver pattern
    ({ $($sender:tt)* }, filter: { |$filter_event:ident| $($filter:tt)* }, map: { |$map_event:ident| $($mapper:tt)* }) => {{
        let task = $crate::emit_stream!({ $($sender)* });
        
        // Use filter_map to process events
        task.filter(|$filter_event| { $($filter)* })
           .map(|$map_event| { $($mapper)* })
           .collect::<Vec<_>>()
           .await
    }};
} 