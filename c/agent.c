#include <emscripten.h>
int hxn_av_decoder_agent(unsigned address) {
  return MAIN_THREAD_EM_ASM_INT({
    if (typeof document === 'undefined') return 0;
    import(new URL('./hxn_av_decoder.mjs', location.href))
      .then(function (agent) { agent.start({memory: wasmMemory, address: $0}); })
      .catch(function (error) { abort('media decoder agent: ' + error); });
    return 1;
  }, address);
}
