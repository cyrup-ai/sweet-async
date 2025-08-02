//! await_final_event macro for handling final event results with OK/ERR sexy syntax
//!
//! This macro provides the syntactic sugar for handling final events in emitting tasks
//! using the elegant OK/ERR pattern syntax.

#[macro_export]
macro_rules! await_final_event {
    // Primary pattern: OK/ERR syntax
    ($task:expr, | $event:ident, $collector:ident | { 
        OK($ok_pat:pat) => $ok_expr:expr,
        ERR($err_pat:pat) => $err_expr:expr 
    }) => {{
        $task.await_final_event(move | $event, $collector | {
            match $event.result() {
                Ok($ok_pat) => $ok_expr,
                Err($err_pat) => $err_expr,
            }
        })
    }};
    
    // With trailing commas
    ($task:expr, | $event:ident, $collector:ident | { 
        OK($ok_pat:pat) => $ok_expr:expr,
        ERR($err_pat:pat) => $err_expr:expr,
    }) => {{
        $task.await_final_event(move | $event, $collector | {
            match $event.result() {
                Ok($ok_pat) => $ok_expr,
                Err($err_pat) => $err_expr,
            }
        })
    }};
    
    // ERR/OK order
    ($task:expr, | $event:ident, $collector:ident | { 
        ERR($err_pat:pat) => $err_expr:expr,
        OK($ok_pat:pat) => $ok_expr:expr 
    }) => {{
        $task.await_final_event(move | $event, $collector | {
            match $event.result() {
                Ok($ok_pat) => $ok_expr,
                Err($err_pat) => $err_expr,
            }
        })
    }};
    
    // ERR/OK with trailing commas
    ($task:expr, | $event:ident, $collector:ident | { 
        ERR($err_pat:pat) => $err_expr:expr,
        OK($ok_pat:pat) => $ok_expr:expr,
    }) => {{
        $task.await_final_event(move | $event, $collector | {
            match $event.result() {
                Ok($ok_pat) => $ok_expr,
                Err($err_pat) => $err_expr,
            }
        })
    }};
    
    // Missing ERR pattern - compile error
    ($task:expr, | $event:ident, $collector:ident | { 
        OK($ok_pat:pat) => $ok_expr:expr
    }) => {{
        compile_error!("await_final_event requires both OK and ERR patterns - missing ERR case")
    }};
    
    // Missing OK pattern - compile error  
    ($task:expr, | $event:ident, $collector:ident | { 
        ERR($err_pat:pat) => $err_expr:expr
    }) => {{
        compile_error!("await_final_event requires both OK and ERR patterns - missing OK case")
    }};
    
    // Empty pattern - compile error
    ($task:expr, | $event:ident, $collector:ident | { }) => {{
        compile_error!("await_final_event requires OK and ERR patterns")
    }};
}