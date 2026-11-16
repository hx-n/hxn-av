//! Writes xavi's binding for hxn and installs its native backend.

use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    xavi_backend::install_scoped(&out, "crate::model").expect("installing the xavi backend");
    let binding = x_idl::hxn::generate(
        xavi_bindgen::NAMESPACE,
        "hxn.av",
        xavi_bindgen::MEDIA_API,
        &xavi_bindgen::browser_idl(),
    )
    .expect("generating the media binding");
    std::fs::write(out.join("media.rs"), binding).expect("writing the media binding");
}
