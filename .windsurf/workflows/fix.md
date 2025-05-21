---
description: Squash all Bugs
---

# Act as an Senior Rust Developer.

```shell
cd ./crates/tokio
```

You will be working exclusively in this directory.

## READ [CONVENTIONS.md](./CONVENTIONS.md) for critical code acceptance criteria

* do not "change directions" from the way the code is working

1. Async patterns - never use `async_trait` or `async fn` in code we own. 
    - Use [sweet_async](https://github.com/cyrup-ai/sweet-mcp.git) crate instead. 
    - Read [SWEET_ASYNC.md](https://github.com/cyrup-ai/sweet_async/docs/SWEET_ASYNC.md) for usage patterns.
  2. If we're in a workspace:
    - use managed [workspace.dependencies] 
    - crates should use: `package_name = { workspace = true }`
  3. 
  4. Don't use .unwrap() outside of tests - handle all Result/Option values properly
  5. Files > 300 lines - decompose into logical concerns
  6. Do not suppress compiler warnings - fix them properly
    - Do not rename with private _ variable names as a means to suppress unused code
    - annotate unused code ONLY if the variable is:
        - publicly exported from the crate 
        - No other member of the crate uses it.
    - Delete code is it is 
        - unused
        - un-necessary
        - unplanned (if it's planned, implement the feature associated)
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

### Code Style

- do not name anything '_wrapper' or '_handler'
    - these are lazy names and non-descriptive
    - rename as helpful, intuitive guidance
- do not add utils or "helpers"
    - this is always a sign of bad code
    - Instead: 
       - use modules 
       - create single purpose files
       - organize logical separation of concerns

### SHOULD YOU GET STUCK on a BUG

- DO NOT MATERIALLY CHANGE ANY DESIGNS OR DIRECTION WITHOUT APPROVAL
- DO NOT REWRITE THE WHOLE IMPLEMENTATION A DIFFERENT WAY
- ASK FOR HELP when you are unsure about direction or when key inflection points are reached.
- ASK FOR HELP if you cannot find or obtain documentation for key libraries.
- ALWAYS TRY TO FIND documentation for key libraries BEFORE asking for help.

## Setup for bugfixing

### Do not try to modify `crates/api/*`

This code is intentially locked. You may suggest changes but unless explicitly granted in writing this code is off-limits except for read operations.

## YOU ARE CONSTRAINED

### No Potential for Improvement

Do not include areas for potential future improvement. If you identify them, think through them with ultrathink, step by step sequential reasoning and roll them into your source code. Do this interatively until there is zero need for a "future enhancements" section.
