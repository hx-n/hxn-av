//! WebCodecs scheduling over the canonical media data/configuration tables.
use crate::{
    browser::request,
    runtime::{ErrorKind, host},
};
use serde_json::{Value, json};
use std::sync::{Arc, LazyLock, Mutex};
use xavi_core::{
    AudioData, AudioDescriptor, AudioSampleFormat, EncodedChunkType,
    handles::{Kind, Slab},
};

struct Decoder {
    capacity: usize,
}
static DECODERS: LazyLock<Mutex<Slab<Decoder>>> =
    LazyLock::new(|| Mutex::new(Slab::new(Kind::MediaCodec)));
fn call<T: Default>(f: impl FnOnce() -> Result<T, String>) -> T {
    match f() {
        Ok(value) => value,
        Err(error) => {
            host::raise(ErrorKind::Runtime, &error);
            T::default()
        }
    }
}
fn capacity(id: i32) -> Result<usize, String> {
    DECODERS
        .lock()
        .map_err(|_| "decoder table lock is poisoned")?
        .get(id)
        .map(|d| d.capacity)
        .ok_or_else(|| "audio decoder is closed".into())
}
fn exact(value: i64) -> Result<i64, String> {
    if value.unsigned_abs() > 9_007_199_254_740_991 {
        Err("timestamp exceeds browser exact integer range".into())
    } else {
        Ok(value)
    }
}
pub fn audio_decoder_create(items: i32, bytes: i64, config: i32) -> i32 {
    call(|| {
        if !(1..=128).contains(&items) || !(16_384..=16 * 1024 * 1024).contains(&bytes) {
            return Err("browser decoder limits require 1..=128 items and 16 KiB..=16 MiB".into());
        }
        let config = crate::MEDIA.configs(config).map_err(|e| e.to_string())?;
        let xavi_backend::pipeline::Configuration::Audio(config) = &*config else {
            return Err("expected audio decoder configuration".into());
        };
        if !(8_000..=96_000).contains(&config.sample_rate)
            || !(1..=2).contains(&config.channels)
            || config.description.len() > 65_536
        {
            return Err("unsupported browser AAC configuration".into());
        }
        let id = DECODERS
            .lock()
            .map_err(|_| "decoder table lock is poisoned")?
            .insert(Decoder {
                capacity: bytes as usize,
            })
            .map_err(|e| e.to_string())?;
        let opened = request(
            json!({"op":"open","id":id,"items":items,"bytes":bytes,"codec":config.codec,"sampleRate":config.sample_rate,"channels":config.channels,"description":&*config.description}),
        );
        if let Err(error) = opened {
            DECODERS.lock().unwrap().remove(id);
            return Err(error);
        }
        Ok(id)
    })
}
pub fn audio_decoder_write(id: i32, value: i32) -> bool {
    call(|| {
        capacity(id)?;
        let chunk = crate::MEDIA.audio_chunk(value).map_err(|e| e.to_string())?;
        let timestamp = exact(chunk.timestamp())?;
        let duration = chunk
            .duration()
            .map(|v| {
                i64::try_from(v)
                    .map_err(|_| "duration exceeds browser range".to_string())
                    .and_then(exact)
            })
            .transpose()?;
        let result = request(
            json!({"op":"write","id":id,"address":chunk.bytes().as_ptr() as u32,"length":chunk.bytes().len(),"timestamp":timestamp,"duration":duration,"kind":match chunk.kind(){EncodedChunkType::Key=>"key",EncodedChunkType::Delta=>"delta"}}),
        )?;
        result
            .as_bool()
            .ok_or_else(|| "invalid decoder write response".into())
    })
}
pub fn audio_decoder_poll(id: i32) -> i32 {
    call(|| {
        capacity(id)?;
        request(json!({"op":"poll","id":id}))?
            .as_i64()
            .filter(|v| (0..=2).contains(v))
            .map(|v| v as i32)
            .ok_or_else(|| "invalid decoder status".into())
    })
}
fn number(value: &Value, key: &str) -> Result<u32, String> {
    value[key]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| format!("invalid decoded {key}"))
}
pub fn audio_decoder_read(id: i32) -> i32 {
    call(|| {
        let mut bytes = vec![0u8; capacity(id)?.min(2048 * 2 * 4)];
        let data = request(
            json!({"op":"read","id":id,"address":bytes.as_mut_ptr() as u32,"capacity":bytes.len()}),
        )?;
        let len = number(&data, "bytes")? as usize;
        if len > bytes.len() {
            return Err("invalid decoder byte count".into());
        }
        let descriptor = AudioDescriptor {
            format: AudioSampleFormat::F32,
            sample_rate: number(&data, "sampleRate")? as f32,
            number_of_frames: number(&data, "frames")?,
            number_of_channels: number(&data, "channels")?,
            timestamp: data["timestamp"]
                .as_i64()
                .ok_or("invalid decoded timestamp")?,
        };
        let audio = AudioData::new(descriptor, &bytes[..len]).map_err(|e| e.to_string())?;
        crate::MEDIA
            .retain_audio(Arc::new(audio))
            .map_err(|e| e.to_string())
    })
}
pub fn audio_decoder_finish(id: i32) {
    call(|| {
        capacity(id)?;
        request(json!({"op":"finish","id":id})).map(|_| ())
    })
}
pub fn audio_decoder_close(id: i32) {
    call(|| {
        capacity(id)?;
        let result = request(json!({"op":"close","id":id}));
        DECODERS
            .lock()
            .map_err(|_| "decoder table lock is poisoned")?
            .remove(id);
        result.map(|_| ())
    })
}
