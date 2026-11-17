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
    // Reuse xavi's CPU operations without forwarding worker-based native codecs.
    let native =
        syn::parse_file(&std::fs::read_to_string(out.join("xavi_backend/native.rs")).unwrap())
            .expect("parsing CPU backend");
    let implemented = native
        .items
        .iter()
        .filter_map(|item| {
            let syn::Item::Fn(f) = item else { return None };
            if !matches!(f.vis, syn::Visibility::Public(_)) {
                return None;
            }
            let name = f.sig.ident.to_string();
            if [
                "audio_encoder_",
                "video_encoder_",
                "video_decoder_",
                "player_",
                "mux",
                "demux",
            ]
            .iter()
            .any(|p| name.starts_with(p))
            {
                return None;
            }
            Some(format!("pub fn {name}() {{}}"))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let web = x_idl::hxn::web_backend(
        xavi_bindgen::NAMESPACE,
        xavi_bindgen::MEDIA_API,
        &xavi_bindgen::browser_idl(),
        &implemented,
    )
    .expect("generating browser CPU forwarders");
    // CPU forwarding needs no IDL wire conversions. Scope descriptors to the
    // shared model rather than hxn's resource wrappers at the crate root.
    let parsed = syn::parse_file(&web).expect("parsing browser forwarders");
    let functions: Vec<_> = parsed
        .items
        .into_iter()
        .filter(|i| matches!(i, syn::Item::Fn(_)))
        .collect();
    let web = quote::quote!(#(#functions)*)
        .to_string()
        .replace("crate ::", "crate :: model ::")
        .replace("crate :: model :: web ::", "crate :: web ::");
    std::fs::write(out.join("media_web.rs"), web).unwrap();
    println!("cargo:rerun-if-changed=page/hxn_av_decoder.mjs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("emscripten") {
        if std::env::var("CARGO_CFG_TARGET_FEATURE")
            .unwrap_or_default()
            .split(',')
            .any(|f| f == "atomics")
        {
            cc::Build::new()
                .file("c/agent.c")
                .flag("-pthread")
                .compile("hxn_av_agent");
        } else {
            let agent = include_str!("page/hxn_av_decoder.mjs")
                .replace("export ", "")
                .replace("''", "\"\"");
            let source = include_str!("c/direct.c").replace("/*AGENT*/", &agent);
            let path = out.join("direct.c");
            std::fs::write(&path, source).unwrap();
            cc::Build::new().file(path).compile("hxn_av_direct");
        }
    }
}
