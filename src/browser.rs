//! Synchronous copied requests; asynchronous codec results stay on the page.
use serde_json::Value;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicU32, Ordering::SeqCst},
};

#[repr(C)]
struct Mailbox {
    sent: AtomicU32,
    done: AtomicU32,
    request: AtomicU32,
    request_len: AtomicU32,
    reply: AtomicU32,
    capacity: AtomicU32,
    reply_len: AtomicU32,
    reserved: AtomicU32,
}
struct Broker {
    mailbox: Box<Mailbox>,
    started: bool,
}
static BROKER: OnceLock<Mutex<Broker>> = OnceLock::new();
#[cfg(target_os = "wasi")]
unsafe extern "C" {
    fn ash_host_agent(name: *const u8, len: u32, address: u32) -> i32;
}
#[cfg(target_os = "emscripten")]
unsafe extern "C" {
    fn hxn_av_decoder_agent(address: u32) -> i32;
}
#[cfg(all(target_os = "emscripten", not(target_feature = "atomics")))]
unsafe extern "C" {
    fn hxn_av_decoder_execute();
}
pub fn request(value: Value) -> Result<Value, String> {
    let broker = BROKER.get_or_init(|| {
        Mutex::new(Broker {
            mailbox: Box::new(Mailbox {
                sent: AtomicU32::new(0),
                done: AtomicU32::new(0),
                request: AtomicU32::new(0),
                request_len: AtomicU32::new(0),
                reply: AtomicU32::new(0),
                capacity: AtomicU32::new(0),
                reply_len: AtomicU32::new(0),
                reserved: AtomicU32::new(0),
            }),
            started: false,
        })
    });
    let mut broker = broker
        .lock()
        .map_err(|_| "browser media transport lock is poisoned")?;
    if !broker.started {
        let address = (&*broker.mailbox) as *const Mailbox as u32;
        #[cfg(target_os = "wasi")]
        let started = unsafe { ash_host_agent(b"hxn_av_decoder".as_ptr(), 14, address) };
        #[cfg(target_os = "emscripten")]
        let started = unsafe { hxn_av_decoder_agent(address) };
        #[cfg(not(any(target_os = "wasi", target_os = "emscripten")))]
        let started = {
            let _ = address;
            0
        };
        if started == 0 {
            return Err("media decoding requires a browser page host".into());
        }
        broker.started = true;
    }
    let bytes = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    let mut reply = vec![0u8; 65_536];
    let m = &broker.mailbox;
    m.request.store(bytes.as_ptr() as u32, SeqCst);
    m.request_len.store(bytes.len() as u32, SeqCst);
    m.reply.store(reply.as_mut_ptr() as u32, SeqCst);
    m.capacity.store(reply.len() as u32, SeqCst);
    #[cfg(all(target_os = "emscripten", not(target_feature = "atomics")))]
    unsafe {
        hxn_av_decoder_execute();
    }
    #[cfg(target_feature = "atomics")]
    {
        let sent = m.sent.fetch_add(1, SeqCst).wrapping_add(1);
        unsafe {
            core::arch::wasm32::memory_atomic_notify(m.sent.as_ptr().cast(), u32::MAX);
        }
        while m.done.load(SeqCst) != sent {
            let seen = m.done.load(SeqCst);
            if seen == sent {
                break;
            }
            unsafe {
                core::arch::wasm32::memory_atomic_wait32(
                    m.done.as_ptr().cast(),
                    seen as i32,
                    1_000_000_000,
                );
            }
        }
    }
    let len = m.reply_len.load(SeqCst) as usize;
    if len > reply.len() {
        return Err("invalid browser media reply length".into());
    }
    let response: Value = serde_json::from_slice(&reply[..len]).map_err(|e| e.to_string())?;
    if let Some(error) = response["error"].as_str() {
        return Err(error.into());
    }
    Ok(response["ok"].clone())
}
