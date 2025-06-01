# META TOKIO IMPLEMENTATION PROMPT

## MISSION: Implement ONE method of Sweet Async API in Tokio runtime

### STRICT CONSTRAINTS

**YOU ARE IMPLEMENTING:** `{TRAIT_NAME}::{METHOD_NAME}`

**EXACT API SIGNATURE:**

```rust
{API_SIGNATURE}
```

**FORBIDDEN ACTIONS:**

- ❌ Creating new traits  
- ❌ Aliasing existing traits
- ❌ Modifying `/api` crate signatures
- ❌ Blaming "previous code"
- ❌ Creating "helper" traits
- ❌ Adding generic bounds not in `/api`
- ❌ Implementing multiple methods at once

**REQUIRED ACTIONS:**

- ✅ Implement ONLY the specified method
- ✅ Use existing `/api` trait signatures EXACTLY
- ✅ Make the test case pass
- ✅ Follow zero-allocation design principles

### IMPLEMENTATION TARGET

**File:** `crates/tokio/src/{TARGET_FILE}`
**Struct:** `{STRUCT_NAME}`
**Method:** `{METHOD_NAME}`

### TEST CASE THAT MUST PASS

```rust
{TEST_CASE}
```

### DESIGN CONSTRAINTS

**Architecture:** {ARCHITECTURE_NOTES}

**Performance:** {PERFORMANCE_REQUIREMENTS}

**Memory:** {MEMORY_CONSTRAINTS}

### SUCCESS CRITERIA

1. Test passes ✅
2. Cargo check passes ✅  
3. Method signature matches `/api` exactly ✅
4. No new traits created ✅
5. Zero allocations in hot path ✅

### IMPLEMENTATION NOTES

{SPECIFIC_IMPLEMENTATION_NOTES}

---

**REMEMBER:** You are implementing ONE method. Nothing else. Do not get creative. Do not abstract. Just make the method work according to the API contract.
