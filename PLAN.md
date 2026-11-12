# Local candidate plan and remaining gates

Completed: exact repository baseline cloned, independent local branch created;
fixed TypeScript contract/handlers/client and official Rust AuthFetch/license read;
typed single-recipient free send/list/ack and device/permission APIs implemented;
contract tests and real TS mutual-auth lifecycle exercised; one non-author review.

Architecture remains independent client → separate MOSAIC thin driver → PayMessage.
No Kernel or server edits, remote push, PR, merge, publication or CI activation.

Follow-up boundaries, not additional mechanisms authorized by this task:

- Real TS and Rust free-operation lifecycle acceptance now passes. Rust normal
  stop/start recovery is tested using an explicit fresh client with the same identity.
  Seamless reuse of the old AuthFetch instance is not claimed.
- Redirect control and expected server identity pin are deployment strategies.
  The pinned SDK has no public pre-request policy/pin parameter; only the verified
  response carries server identity. No custom crypto or policy framework is added.
- Certificate exchange, adversarial canonicalization, cancellation, signed 402,
  streaming caps and production MySQL are outside this tested free-operation subset.

Detailed evidence: VALIDATION.md and interop/RUST_SERVER.md. Experimental adapter
status remains; no remote publication or change to the accepted server scope.
