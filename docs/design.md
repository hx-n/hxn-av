# Design

The xavi declaration and scoped CPU backend remain the source of the media
API. x-idl emits one shared model and hxn exports the same classes on every
runtime. Native codecs keep their existing platform workers. Browser
forwarders expose CPU operations and AudioDecoder only; unsupported native
worker operations raise rather than starting threads in an unthreaded build.

Browser AudioDecoder uses WebCodecs on the page. Rust retains codec configs
and encoded chunks in the existing MediaBackend tables. A library-wide
mailbox copies commands synchronously and acknowledges only after input
bytes are copied. Decoded blocks stay in a bounded page queue until a read
copies f32 PCM into a caller-owned destination during that same synchronous
request. The returned AudioData enters the existing media table, retaining
its ordinary conversion, slicing, equalizer and close behavior.

Input item/byte credits include queued codec work. Output item/byte limits
include retained decoded blocks; conservative AAC block reservations stop
input before output fills. Unexpected output exceeding those bounds becomes
a terminal decoder error, never a silent dropped frame. Finish is a flush
barrier: Ended appears only after flush resolves and every output is read.
Close invalidates page callbacks, closes WebCodecs and releases queued data.

Ash starts the agent through ash_host_agent. Threaded Emscripten starts it
on the page thread and serves shared-memory requests; unthreaded Emscripten
embeds the same agent and dispatches synchronously. No asynchronous callback
writes guest request/reply memory. The package init macro installs its agent
beside each wasm program, including direct haxe -lib use.

WebCodecs configuration follows the [AAC codec registration](https://www.w3.org/TR/webcodecs-aac-codec-registration/):
AudioSpecificConfig describes raw AAC packets; absent description selects
ADTS packets. Decoder availability depends on the browser.

Emscripten packages reserve a 1 MiB program/pthread stack for CPU media
operations, including the equalizer's fixed filter state. This also covers
the larger temporary values in debug builds. On Apple hxcpp static builds,
the init macro preserves system dependencies as archive auto-link references.
Android hxcpp consumers target API 28 or later, matching the native codec
backend's NDK declarations.
