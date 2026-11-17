// Codec results remain page-owned until a synchronous read copies them to the guest.
const decoders = new Map();
const encoder = new TextEncoder();
const text = new TextDecoder();
function failure(state, error) {
  if (state.closed) return;
  state.error = String(error);
  state.queue.length = 0;
  state.bytes = 0;
  try { state.decoder.close(); } catch (_) {}
}
function get(id) {
  const state = decoders.get(id);
  if (!state || state.closed) throw new Error('audio decoder is closed');
  if (state.error) throw new Error(state.error);
  return state;
}
function execute(memory, request) {
  const {op, id} = request;
  if (op === 'open') {
    if (!globalThis.AudioDecoder) throw new Error('WebCodecs AudioDecoder is unavailable');
    if (decoders.size >= 32 || decoders.has(id)) throw new Error('audio decoder table is full');
    if (request.codec !== 'aac' && request.codec !== 'mp4a.40.2') throw new Error('browser audio decoding supports AAC-LC');
    const state = {closed:false,error:'',queue:[],bytes:0,reserved:0,inputBytes:0,pending:[],finished:false,flushed:false,items:request.items,limit:request.bytes,block:2048*request.channels*4,decoder:null};
    state.decoder = new AudioDecoder({
      output(data) {
        try {
          if (state.closed || state.error) return;
          const size = data.allocationSize({planeIndex:0,format:'f32'});
          if (data.numberOfChannels !== request.channels || data.sampleRate !== request.sampleRate) throw new Error('decoder output format changed');
          if (size > state.block || state.queue.length >= state.items || state.bytes + size > state.limit) throw new Error('decoded audio exceeds bounded output capacity');
          const samples = new Float32Array(size / 4);
          data.copyTo(samples, {planeIndex:0,format:'f32'});
          if (!Number.isSafeInteger(data.timestamp)) throw new Error('decoded timestamp exceeds exact integer range');
          state.queue.push({samples,frames:data.numberOfFrames,channels:data.numberOfChannels,sampleRate:data.sampleRate,timestamp:data.timestamp});
          state.bytes += size;
          state.reserved = Math.max(0, state.reserved - 1);
          state.inputBytes -= state.pending.shift() || 0;
        } catch (error) { failure(state,error); }
        finally { data.close(); }
      },
      error(error) { failure(state,error); }
    });
    try {
      const config = {codec:'mp4a.40.2',sampleRate:request.sampleRate,numberOfChannels:request.channels};
      if (request.description.length) config.description = new Uint8Array(request.description);
      state.decoder.configure(config);
      decoders.set(id,state);
    } catch (error) { state.decoder.close(); throw error; }
    return {};
  }
  if (op === 'close') {
    const state = decoders.get(id);
    if (state) { state.closed=true;state.queue.length=0;state.bytes=0;try {state.decoder.close();} catch (_) {}decoders.delete(id); }
    return {};
  }
  const state = get(id);
  if (op === 'write') {
    if (state.finished) throw new Error('audio decoder input has finished');
    if (request.length > state.limit) throw new Error('encoded chunk exceeds decoder byte capacity');
    if (state.inputBytes + request.length > state.limit || state.reserved + state.queue.length >= state.items || state.bytes + (state.reserved+1)*state.block > state.limit) return false;
    if (!Number.isSafeInteger(request.timestamp) || (request.duration != null && !Number.isSafeInteger(request.duration))) throw new Error('encoded timestamp or duration exceeds exact integer range');
    const data = new Uint8Array(memory.buffer,request.address,request.length).slice();
    const init = {type:request.kind,timestamp:request.timestamp,data};
    if (request.duration != null) init.duration=request.duration;
    state.decoder.decode(new EncodedAudioChunk(init));
    state.reserved++; state.inputBytes+=data.length; state.pending.push(data.length);
    return true;
  }
  if (op === 'finish') {
    if (!state.finished) {
      state.finished=true;
      state.decoder.flush().then(() => {if(!state.closed&&!state.error){state.flushed=true;state.reserved=0;state.pending.length=0;state.inputBytes=0;}}).catch(error=>failure(state,error));
    }
    return {};
  }
  if (op === 'poll') return state.queue.length ? 1 : state.flushed ? 2 : 0;
  if (op === 'read') {
    const block = state.queue[0];
    if (!block) throw new Error('audio decoder output is not ready');
    const bytes = new Uint8Array(block.samples.buffer);
    if (bytes.length > request.capacity) throw new Error('audio decoder destination is too small');
    new Uint8Array(memory.buffer,request.address,bytes.length).set(bytes);
    state.queue.shift(); state.bytes-=bytes.length;
    return {bytes:bytes.length,frames:block.frames,channels:block.channels,sampleRate:block.sampleRate,timestamp:block.timestamp};
  }
  throw new Error('unknown audio decoder operation');
}
export function start({memory,address,serveMailbox=true}) {
  const dispatch=()=>{
    const words=new Int32Array(memory.buffer,address,8);
    let response;
    try {
      const request=JSON.parse(text.decode(new Uint8Array(memory.buffer,words[2]>>>0,words[3]>>>0).slice()));
      response={ok:execute(memory,request)};
    } catch(error) {response={error:String(error)};}
    let bytes=encoder.encode(JSON.stringify(response));
    if(bytes.length>(words[5]>>>0)) bytes=encoder.encode('{"error":"media reply exceeds capacity"}');
    new Uint8Array(memory.buffer,words[4]>>>0,bytes.length).set(bytes);words[6]=bytes.length;
  };
  if(serveMailbox)(async()=>{
    let seen=0;
    for(;;){
      const words=new Int32Array(memory.buffer,address,8);
      if(Atomics.waitAsync){const wait=Atomics.waitAsync(words,0,seen);if(wait.async)await wait.value;}
      else while(Atomics.load(words,0)===seen)await new Promise(resolve=>setTimeout(resolve,1));
      const sent=Atomics.load(words,0);if(sent===seen)continue;
      dispatch();seen=sent;Atomics.store(words,1,seen);Atomics.notify(words,1);
    }
  })();
  return dispatch;
}
