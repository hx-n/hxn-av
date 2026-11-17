# Conventions

- Reuse xavi's model, resource tables and PCM conversions. Codec work belongs
  in this library; device mixing belongs in hxn-audio.
- Keep every browser queue bounded by items and bytes, with explicit retry
  when full. Flush/EOF must preserve every accepted packet's output. Close
  must release pending outputs and invalidate late callbacks.
- Copy guest inputs before acknowledging requests. Guest destinations may
  be written only synchronously during the read request, never by a codec
  callback. Validate browser integer timestamp limits before conversion.
- Do not start native worker threads for unsupported browser operations.
  Fail unsupported operations through the ordinary Haxe exception path.
- Test native HashLink, Ash hybrid/JIT/interpreter, hxcpp and CPPIA. Test
  browser CPU/decoder behavior on Ash wasm and both Emscripten modes under
  timeouts, repeating asynchronous tests. Test mobile after linking changes.
- Source cross-builds require hxn to filter host build-dependency linker
  metadata from target packages. CI and releases use matching hxn tool/crate
  versions. Test the installed haxelib as well as source builds.
- Update design, skills and README with supported API/workflow changes.
  Keep measurements/status in the parent hxn git-bug tracker. Format Haxe
  and Rust before a commit; commit/push only when asked.
