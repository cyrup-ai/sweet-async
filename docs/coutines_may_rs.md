# Coroutines

## May (Rust)

### Objective: Implement a high-performance echo server using stackful coroutines with the May library

  **Solution:**

    ```rust
    #[macro_use]
    extern crate may;

    use may::net::TcpListener;
    use std::io::{Read, Write};

    fn main() {
        let listener = TcpListener::bind("127.0.0.1:8000").unwrap();
        while let Ok((mut stream, _)) = listener.accept() {
            go!(move || {
                let mut buf = vec![0; 1024 * 16]; // alloc in heap!
                while let Ok(n) = stream.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    stream.write_all(&buf[0..n]).unwrap();
                }
            });
        }
    }
    ```

## May Coroutine Examples - Use Cases and Patterns

### OBJECTIVE: Non-blocking I/O operations with stdin/stdout

#### SOLUTION: Use CoIo wrappers for standard streams**

    ```rust
    // From general_io.rs
    let mut stdin = CoIo::new(io::stdin()).expect("failed to create stdio");
    let mut stdout = CoIo::new(io::stdout()).expect("failed to create stdout");
    ```

*Why use this pattern:* When you need to read/write to standard streams without blocking the entire thread. Traditional stdin/stdout operations would block the thread, but CoIo enables asynchronous I/O within the coroutine model.

### OBJECTIVE: Single-threaded coroutine scheduling

#### SOLUTION: Configure May to use exactly one worker thread

    ```rust
    // From single_thread_schedule.rs
    may::config().set_workers(1);
    ```

*Why use this pattern:* Eliminates thread context switching overhead for scenarios where CPU parallelism isn't needed. This provides better performance for I/O-bound workloads and avoids synchronization costs between threads.

### OBJECTIVE: Create high-performance UDP server

#### SOLUTION: Use May coroutines with UDP sockets

    ```rust
    // From echo_udp.rs
    let sock = UdpSocket::bind(("0.0.0.0", port)).unwrap();
    for _ in 0..threads {
        let sock = sock.try_clone().unwrap();
        go!(move || {
            let mut buf = vec![0u8; 1024 * 16];
            loop {
                let (len, addr) = sock.recv_from(&mut buf).unwrap();
                sock.send_to(&buf[0..len], addr).unwrap();
            }
        });
    }
    ```

*Why use this pattern:* Provides high concurrency without thread overhead. Each UDP connection gets its own coroutine, which is much lighter than threads and scales to thousands of connections.

### OBJECTIVE: Create secure HTTPS server

#### SOLUTION: Combine May's TCP listener with TLS

    ```rust
    // From https.rs
    let identity = Identity::from_pkcs12(&identity, "password").unwrap();
    let acceptor = TlsAcceptor::new(identity).unwrap();
    let acceptor = Arc::new(acceptor);

    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    while let Ok((stream, _)) = listener.accept() {
        let acceptor = acceptor.clone();
        let mut stream = acceptor.accept(stream).unwrap();
        go!(move || { /* handle request */ });
    }
    ```

*Why use this pattern:* Enables secure connections with TLS while maintaining the high concurrency model of coroutines. Compared to thread-based TLS servers, this approach scales to many more connections.

### OBJECTIVE: Demonstrate parent-child coroutine relationships

#### SOLUTION: Use coroutine scopes for structured concurrency

    ```rust
    // From spawn.rs
    let h = go!(move || {
        println!("hi, I'm parent");
        let v = (0..100)
            .map(|i| {
                go!(move || {
                    println!("hi, I'm child{i}");
                    yield_now();
                    println!("bye from child{i}");
                })
            })
            .collect::<Vec<_>>();
        yield_now();
        // wait child finish
        for i in v {
            i.join().unwrap();
        }
        println!("bye from parent");
    });
    ```

*Why use this pattern:* Provides structured concurrency with clear parent-child relationships. This helps manage the lifecycle of concurrent operations and prevents resource leaks.

### OBJECTIVE: Handle multiple event sources simultaneously

#### SOLUTION: Use select! macro for multiplexing**

    ```rust
    // From select.rs
    let id = select!(
        _ = listener.accept() => println!("got connected"),
        _ = coroutine::sleep(Duration::from_millis(1000)) => {},
        _ = rx1.recv() => println!("rx1 received"),
        a = rx2.recv() => println!("rx2 received, a={a:?}")
    );
    ```

*Why use this pattern:* Efficiently waits for multiple event sources without blocking or polling. This is more efficient than checking each source in a loop and cleaner than using callbacks.

### OBJECTIVE: Create a simple HTTP server

#### SOLUTION: Use coroutines to handle HTTP requests concurrently

    ```rust
    // From http.rs
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    while let Ok((mut stream, _)) = listener.accept() {
        go!(move || {
            let mut buf = Vec::new();
            let mut path = String::new();
            // Process HTTP request
        });
    }
    ```

*Why use this pattern:* Handles many concurrent HTTP connections with minimal resource usage. Each connection gets its own coroutine that efficiently yields when waiting for I/O.

### OBJECTIVE: Create a WebSocket server

#### SOLUTION: Combine May with WebSocket library

    ```rust
    // From websocket.rs
    let listener = TcpListener::bind(("0.0.0.0", 8080)).unwrap();
    for stream in listener.incoming() {
        go!(move || {
            let mut websocket = accept(stream.unwrap()).unwrap();
            loop {
                let msg = websocket.read().unwrap();
                if msg.is_binary() || msg.is_text() {
                    websocket.send(msg).unwrap();
                }
            }
        });
    }
    ```

*Why use this pattern:* Enables WebSocket communication with high concurrency. The coroutine model is well-suited for the long-lived connections typical in WebSocket applications.

### OBJECTIVE: Implement advanced event processing with ordering guarantees

#### SOLUTION: Use cqueue for structured event handling

    ```rust
    // From cqueue.rs
    cqueue::scope(|cqueue| {
        // Register event sources
        for t in 0..10 {
            go!(cqueue, t, |es| {
                // Process events
                es.send(0);
            });
        }

        // Poll and process events in order
        while let Ok(ev) = cqueue.poll(None) {
            // Process event sequentially
        }
    });
    ```

*Why use this pattern:* Provides more control over event processing order compared to basic coroutines. This is useful when events need to be processed in a specific sequence or with shared state.

### OBJECTIVE: Continuously multiplex operations in a loop

#### SOLUTION: Use loop_select! for repeated event monitoring

    ```rust
    // From loop_select.rs
    may::loop_select!(
        v1 = rx1.recv() => println!("v1={:?}", v1),
        v2 = rx2.recv() => println!("v2={:?}", v2)
    );
    ```

*Why use this pattern:* Simplifies continuous event monitoring without manual loop construction. This is cleaner than writing your own select loop and handles channel closure properly.

### OBJECTIVE: Create high-performance TCP benchmarking tool

#### SOLUTION: Use coroutines for concurrent connection testing

    ```rust
    // From echo_client.rs
    for _ in 0..test_conn_num {
        go!(scope, || {
            let mut conn = TcpStream::connect(addr).unwrap();
            conn.set_nodelay(true).unwrap();

            loop {
                conn.write_all(&msg).unwrap();
                out_num.fetch_add(1, Ordering::Relaxed);
                // Read response and track metrics
            }
        });
    }
    ```

*Why use this pattern:* Tests server performance with many concurrent connections without the overhead of threads. This allows for more realistic load testing with thousands of connections.
