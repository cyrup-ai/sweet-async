---
description: Squash all Bugs
---

# Act as an Senior Rust Developer.

## READ CONVENTIONS.md for critical code acceptance criteria



- do not "change directions" from the way the code is working

1. Async patterns - never use `async_trait` or `async fn` in code we own. Use [sweet_async](https://github.com/cyrup-ai/sweet-mcp.git) crate instead. Read [SWEET_ASYNC.md](./docs/SWEET_ASYNC.md) for usage patterns.
  2. If we're in a workspace:
    - use managed [workspace.dependencies] 
    - crates should use: `package_name = { workspace = true }`
  3. 
  4. Using .unwrap() outside of tests - handle all Result/Option values properly
  5. Files > 300 lines - decompose into smaller modules
  6. Suppressing compiler warnings - fix them properly
  7. Adding features not requested - minimal implementation only
  8. Leaving TODOs about "real world" implementation - write production code now
  9. Missing appropriate error handling - use custom error types
  10. Not reading latest docs before implementation - get updated synta
----------------------------------------------------------

## YOUR TASK: Fix all the WARNINGS and errors

1. Recursively fix all warnings and errors until:
   - `cargo check --message-format short --quiet` shows zero errors.
   - All necessary functionality works properly.

2. Evaluation Step:
   - If no research is needed, proceed to implement REAL code (never just stubs or simulations).
   - Validate your changes by interacting with the application as an end user would:
     - Build and run the main binary (for example: `cargo run`).
     - Provide real input/output examples that demonstrate the feature works.
   - If research is required to address unknown or complex issues:
     - look in ./docs/**/*.md to see if we already have docs on the library/version in question
     - Use these tools to get answers:
        - github_mcp_server
        - context7 mcp
        - firecrawl mcp
        - web search
      - don't give up on research after 1 or 2 failure
          - sometimes tools fail
          - often your search query is WAY TOO SPECIFIC 
             - or 
          - makes assumptions in the approach to solution
4. Testing:
   - Never write unit tests in the source files.
   - All unit tests belong in the `tests` directory, using `nextest`

5. Additional Rules:
   - Provide the command used to verify each feature using the main application interface.
     - Include the example input and corresponding output logs to prove the feature works.


Remember: Keep iterating until the project is bug-free, thoroughly tested, and production-ready!

_Pro tip_: order all warnings and errors by module. rank sort by highest count. Fixing the highest count module often fixes many more outside so you get a huge reduction in count.

### SHOULD YOU GET STUCK on a BUG

- DO NOT MATERIALLY CHANGE ANY DESIGNS OR DIRECTION WITHOUT APPROVAL
- DO NOT REWRITE THE WHOLE IMPLEMENTATION A DIFFERENT WAY
- ASK FOR HELP and 
