//! xavi's `media` API for Haxe through hxn: x-idl writes the binding from
//! xavi's declaration, over its CPU backend and platform codecs.

#![allow(clippy::should_implement_trait)]
#![cfg_attr(
    all(target_arch = "wasm32", target_feature = "atomics"),
    feature(stdarch_wasm_atomic_wait)
)]

#[cfg(target_family = "wasm")]
mod browser;
#[cfg(target_family = "wasm")]
mod decoder;

#[allow(dead_code, clippy::all)]
mod native {
    include!(concat!(env!("OUT_DIR"), "/xavi_backend/native.rs"));
}

#[cfg(not(target_family = "wasm"))]
use native as backend;

#[cfg(target_family = "wasm")]
mod web {
    pub use crate::decoder::{
        audio_decoder_close, audio_decoder_create, audio_decoder_finish, audio_decoder_poll,
        audio_decoder_read, audio_decoder_write,
    };
    pub use crate::native::*;
}
#[cfg(target_family = "wasm")]
#[allow(dead_code, unused_variables, unused_unsafe, clippy::all)]
mod backend {
    use crate::model::*;
    include!(concat!(env!("OUT_DIR"), "/media_web.rs"));
}

include!(concat!(env!("OUT_DIR"), "/media.rs"));

// The backend's handle tables, shared by every runtime that loads the library.
static MEDIA: xavi_backend::MediaBackend = xavi_backend::MediaBackend::new();

fn with_media<T>(operation: impl FnOnce(&xavi_backend::MediaBackend) -> T) -> T {
    operation(&MEDIA)
}
