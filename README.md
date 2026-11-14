# hxn-av

[![Ash](https://github.com/hx-n/hxn-av/actions/workflows/ash.yml/badge.svg)](https://github.com/hx-n/hxn-av/actions/workflows/ash.yml)
[![HashLink](https://github.com/hx-n/hxn-av/actions/workflows/hashlink.yml/badge.svg)](https://github.com/hx-n/hxn-av/actions/workflows/hashlink.yml)
[![hxcpp](https://github.com/hx-n/hxn-av/actions/workflows/hxcpp.yml/badge.svg)](https://github.com/hx-n/hxn-av/actions/workflows/hxcpp.yml)
[![CPPIA](https://github.com/hx-n/hxn-av/actions/workflows/cppia.yml/badge.svg)](https://github.com/hx-n/hxn-av/actions/workflows/cppia.yml)

Audio and video for Haxe on HashLink, Ash, hxcpp and CPPIA: video frames
and PCM audio, an equalizer, native encoders and decoders, MP4 reading and
writing, and a file player. The codecs are the platform's own, with no
FFmpeg. The same program compiles for each runtime.

```haxe
import haxe.Int64;
import haxe.io.Bytes;
import media.*;

class Main {
	static function main() {
		// A quiet 440 Hz mono tone, sampled at 48 kHz.
		var pcm = Bytes.alloc(1024 * 4);
		for (i in 0...1024)
			pcm.setFloat(i * 4, 0.1 * Math.sin(2 * Math.PI * 440 * i / 48000));
		var init = new AudioDataInit(AudioSampleFormat.F32, 48000, Int64.ofInt(1024), Int64.ofInt(1), Int64.ofInt(0), pcm);
		var audio = AudioData.create(init);
		init.close();

		var eq = AudioEqualizer.create(1);
		eq.setBand(0, 440, 6, 1); // Band index, frequency, gain in dB, Q.
		eq.setPreamp(-6);
		var filtered = eq.process(audio);
		trace(filtered.numberOfFrames());
		filtered.close();
		audio.close();
		eq.close();
	}
}
```

## Installing

```
haxelib install hxn-av
```

Then compile with `-lib hxn-av`, for HashLink or hxcpp:

```
haxe -lib hxn-av -main Main -hl main.hl
haxe -lib hxn-av -main Main -cpp out
```

The haxelib contains the generated classes and prebuilt native libraries for
Linux x86-64, macOS arm64 and Windows x86-64, so a program needs no Rust. For
HashLink, the native library is copied next to the `.hl` output at compile
time. Each GitHub release also has the haxelib as a ZIP, which
`haxelib install hxn-av-<version>.zip` installs.

The generated classes are also committed to this repository, so the API can
be read here, but the native libraries are only in releases: a `haxelib git`
install compiles once a release's `native/` directory is copied into it.

CI runs each runtime on Linux, macOS and Windows, except HashLink, which runs
on Linux and Windows. Android, iOS and wasm builds are not set up yet.

## Classes

All in the package `media`:

| Class | |
|---|---|
| `MediaPlayer` | Plays a file with its sound, and hands over video frames to draw: pause, seek, volume and an equalizer. |
| `VideoFrame` | Pixels in CPU memory, with their layout and timing: copy, crop, resize, blend and retime. |
| `AudioData` | PCM samples: format conversion, copy, slice, gain, mix and retime. |
| `AudioEqualizer` | Up to 16 peaking bands, a preamp and bypass, for `AudioData` or a playing `MediaPlayer`. |
| `AudioEncoder`, `VideoEncoder` | Native encoding, a frame or a block of samples at a time. |
| `AudioDecoder`, `VideoDecoder` | Native decoding, polled for output. |
| `MediaDemuxer`, `MediaMuxer` | Reading and writing MP4 files, a packet at a time. |
| `MediaQueue` | A bounded queue of samples, frames, packets or bytes, which refuses a write when full. |

Every object holds a native resource until `close` is called; the garbage
collector does not release it. Timestamps and durations are in
microseconds. `VideoFrame.copyTo` returns an `hxn.Future`, which `await`
waits for. Failures are thrown as Haxe exceptions.

The codecs are AAC-LC and H.264 Baseline: AVFoundation and VideoToolbox on
macOS, Media Foundation on Windows, and the system's GStreamer 1.20 or later
on Linux, which needs its codec and audio output plugins installed. The
frame, sample and equalizer classes need none of them.

## Examples

- `examples/check`: the test CI runs, which makes a video frame and copies
  it back, then filters a tone through an equalizer. It needs no audio or
  video device.

## How the binding is generated

The API is declared in [xavi](https://github.com/rayzor-blade/xavi), along
with its native backend. `build.rs` installs the backend, and
[x-idl](https://github.com/rayzor-blade/x-idl)'s hxn target writes the
binding from the declaration. `hxn build` runs the build script first, then
generates each runtime's adapter and one set of Haxe classes, shared by
every runtime, from that binding.
Changing the API means changing xavi's declaration and updating the xavi
revision this crate pins.

## Building from source

Requires the [hxn](https://github.com/hx-n/hxn) CLI, Rust and Haxe 4.3.
`hxn check` builds the binding and runs `examples/check` on each runtime
named, and fails unless every run passes and prints the same output.
`hxn package` writes the haxelib for this machine:

```
hxn check hl --vm ash
hxn check cpp cppia
hxn package hl cpp --out ../hxn-av-haxelib
```

To build against checkouts of hxn, x-idl and xavi beside this one, put
`[patch]` tables for their git URLs in `.cargo/config.toml`, which git
ignores.

## License

Apache-2.0. x-idl is MIT.
