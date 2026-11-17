#include <emscripten.h>
EM_JS(int, hxn_av_decoder_start_js, (unsigned address), {
  if (typeof document === 'undefined') return 0;
  /*AGENT*/
  Module.hxnAvDecoderExecute = start({memory: {get buffer() { return HEAPU8.buffer; }}, address, serveMailbox: false});
  return 1;
});
EM_JS(void, hxn_av_decoder_execute_js, (), { Module.hxnAvDecoderExecute(); });
int hxn_av_decoder_agent(unsigned address) { return hxn_av_decoder_start_js(address); }
void hxn_av_decoder_execute() { hxn_av_decoder_execute_js(); }
