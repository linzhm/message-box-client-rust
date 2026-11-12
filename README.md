# message-box-client-rust

An independent Rust MessageBox client project, initialized as a private repository.

**Status: documentation only.** There is no Rust crate, implementation, dependency,
workflow, or interoperability result yet. The sections below describe intended scope.

## Architecture

1. **Rust MessageBox Client (this repository):** independent of Kernel and MOSAIC.
   Follow the selected server's standard OpenAPI contract and authentication protocol
   for message send, list/receive, and acknowledgement. Resolve the authoritative
   contract before implementation; this initialization defines no new API or paths.
2. **MOSAIC thin driver (in MOSAIC):** reuse this client and map operations to file
   resources according to MOSAIC's existing specifications. No driver belongs here.
3. **PayMessage (upper application layer):** build application behavior above those
   layers. Existing Socket.IO transport is not PayMessage. Realtime adaptation is
   a separate concern from the initial request/response client.

See [PLAN.md](PLAN.md) for the limited follow-up and upstream review. This repository
does not change the server project's implementation, release scope, or acceptance.

## License

Original initialization documents are reserved under [LICENSE](LICENSE). No
third-party source is included and no upstream dependency is selected. Referenced
projects retain their own licenses; future reuse must preserve their requirements.
