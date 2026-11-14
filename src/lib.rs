//! xavi's `media` API for Haxe through hxn: x-idl writes the binding from
//! xavi's declaration, over xavi's native backend.

#[allow(dead_code)]
mod backend {
    include!(concat!(env!("OUT_DIR"), "/xavi_backend/native.rs"));
}

include!(concat!(env!("OUT_DIR"), "/media.rs"));

// The backend's handle tables, shared by every runtime that loads the library.
static MEDIA: xavi_backend::MediaBackend = xavi_backend::MediaBackend::new();

fn with_media<T>(operation: impl FnOnce(&xavi_backend::MediaBackend) -> T) -> T {
    operation(&MEDIA)
}
