# Task: SenderEvent<T> Implementation

## 🚨 API PROTECTION CONSTRAINT 🚨
**NO MODIFICATION OF ./api IS EVER ALLOWED!!! The work is IMPLEMENTING the api precisely NEVER changing the api!!!**

## Description
Implement sender-side event abstractions for data producers in streaming workflows.

## Success Criteria
- [ ] Implement SenderEvent<T> trait for producer events
- [ ] Add event generation, batching, and emission controls  
- [ ] Support backpressure handling and flow control
- [ ] Implement proper error handling for failed sends
- [ ] Add sender-side metrics and monitoring hooks

## Dependencies
- 2_streaming/2_streaming_event.md (base event abstraction)
- 4_sender_receiver/0_sender_task.md (sender implementation)

## Files to Implement/Fix
- tokio/src/task/emit/event.rs (add SenderEvent)

## Complexity  
Medium - Producer-side event handling with flow control

## Production Impact
High - Used in all data generation patterns
## Implementation Constraints

Write code with these goals in mind: 

  - zero allocation
  - blazing-fast
  - no unsafe
  - no unchecked 
  - *no locking*
  - elegant ergonomic code

DO NOT WRITE TESTS IN THE SAME FILE
ANOTHER AGENT will write those in ./tests/ (sister to src)


Do not include areas for potential future improvement. If you identify them, think through them with ultrathink, step by step sequential reasoning and roll them into your source code. Do this iteratively and recursively until there is zero need for a "future enhancements" section.

think sequentially. step by step. ULTRATHINK.

Check all your work twice to ensure no symbol, method, trait bounds or other detail is missed, misaligned or omitted.

Review the architecture and requirements ... Focus keenly on the USER OBJECTIVE. it is your "guiding light" and ultimate "source of truth". Ensure all delivered items incrementally lead to this end state and ALL "the pieces fit.

Check all of your work a third time. Think sequentially, step by step. ULTRATHINK. Focus on performance. Are you using channels properly. are you optimizing allocations and inlining all the happy paths where it wi matter. Are all errors handled fully and semantically? think sequentially. step by step. ULTRATHINK.

Check all of your work a fourth time. think sequentially. step by step. ULTRATHINK. "Have I provided ALL the code, full and complete with all details handled and no "future enhancements", todos, "in a real situation", "for now", "in production". All such work will be rejected. Revise it recursively until it is perfected. 

Check all your work a fifth time. Are all the third party libraries using the very latest api signatures and "best in class idioms"? Revise your work recursively until all such issues are handled. Be a software artisan. Complex, feature rich, elegant, ergonomic source code is your requirement.

## All Issues Handle. NOTHING simplified. NOTHING stubbed. NOTHING "miminal"

Do not include areas for potential future improvement. If you identify them, think through them with ultrathink, step by step sequential reasoning and roll them into your source code. Do this interactively until there is zero need for a "future enhancements" section.

=========================================

- express all source code fully
- certify that the code is complete and every potential optimization is included.


==== MANIFEST WITH THESE CONSTRAINTS =====

## No Potential for Improvement

Do not include areas for potential future improvement. If you identify them, think through them with ultrathink, step by step sequential reasoning and roll them into your source code. Do this iteratively and recursively until there is zero need for a "future enhancements" section.

ADDITIONAL CONSTRAINTS:

- never use unwrap() (period!)
- never use expect() (in src/* or in examples)
- DO USE expect() in ./tests/*
- DO NOT use unwrap in ./tests/*

## MAKE ONLY NECESSARY CHANGES

- Focus on the User's objective
- Be useful, not thorough
- Make surgical, targeted changes vs sweeping changes

## DO NOT STUB CODE TO COME BACK LATER

- You will forget! 
- Write the full and correct code right now!
- if you don't know how and need to research, pause and research

## CLARIFICATIONS 

I DO NOT WANT YOU TO REWRITE WORKING CODE UNLESS REQUESTED (Bad)
I DO WANT YOU TO WRITE ALL NEW AND MODIFIED CODE WITH THESE CONSTRAINTS 