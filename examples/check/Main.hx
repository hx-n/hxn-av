import haxe.Int64;
import haxe.io.Bytes;
import hxn.av.*;

/**
	Makes a video frame and copies it back out, then filters a tone through an
	equalizer and copies the samples back. It needs no audio or video device.
	`hxn check` runs it on each runtime: it exits with 1 on a failed check, and
	every runtime must print the same output.
**/
#if emscripten
@:cppFileCode("#include <emscripten.h>\n#include <emscripten/eventloop.h>")
#end
class Main {
	static function check(holds:Bool, what:String) {
		Sys.println('$what: ${holds ? "yes" : "no"}');
		if (!holds)
			Sys.exit(1);
	}

	static function main() {
		#if emscripten
		untyped __cpp__("emscripten_runtime_keepalive_push()");
		#end
		// A 2 × 2 opaque red RGBA frame.
		var pixels = Bytes.alloc(16);
		for (i in 0...4) {
			pixels.set(i * 4, 255);
			pixels.set(i * 4 + 3, 255);
		}
		var init = new VideoFrameBufferInit(VideoPixelFormat.RGBA, Int64.ofInt(2), Int64.ofInt(2), Int64.ofInt(0));
		var frame = VideoFrame.create(pixels, init);
		init.close();
		check(frame.format() == VideoPixelFormat.RGBA, "frame is RGBA");
		check(frame.codedWidth() == 2 && frame.codedHeight() == 2, "frame is 2 x 2");

		var options = new VideoFrameCopyToOptions();
		var size = frame.allocationSize(options);
		check(size == 16, "frame takes 16 bytes");
		var copy = Bytes.alloc(Int64.toInt(size));
		final copied = frame.copyTo(copy, options);
		if (!copied.isReady())
			throw "CPU video copy did not resolve synchronously";
		finish(frame, pixels, options, copy, copied.await());
	}

	static function fail(error:Dynamic):Void {
		Sys.println('media check failed: ${Std.string(error)}');
		leave(1);
	}

	public static function leave(code:Int):Void {
		#if emscripten
		untyped __cpp__("emscripten_runtime_keepalive_pop(); emscripten_force_exit({0})", code);
		#else
		hxn.Loop.exit(code);
		#end
	}

	static function finish(frame:VideoFrame, pixels:Bytes, options:VideoFrameCopyToOptions, copy:Bytes, layouts:PlaneLayouts):Void {
		options.close();
		check(layouts.count() == 1 && layouts.stride(0) == 8, "copy has one plane of 8-byte rows");
		check(copy.compare(pixels) == 0, "copy matches the frame");
		layouts.close();

		var larger = frame.resize(Int64.ofInt(4), Int64.ofInt(4));
		check(larger.codedWidth() == 4, "resized to 4 x 4");
		larger.close();
		frame.close();

		// A quiet 440 Hz mono tone, sampled at 48 kHz.
		var frames = 1024;
		var pcm = Bytes.alloc(frames * 4);
		for (i in 0...frames)
			pcm.setFloat(i * 4, 0.1 * Math.sin(2 * Math.PI * 440 * i / 48000));
		var audioInit = new AudioDataInit(AudioSampleFormat.F32, 48000, Int64.ofInt(frames), Int64.ofInt(1), Int64.ofInt(0), pcm);
		var audio = AudioData.create(audioInit);
		audioInit.close();
		check(audio.numberOfFrames() == frames && audio.sampleRate() == 48000, "tone has 1024 frames at 48 kHz");

		var eq = AudioEqualizer.create(1);
		eq.setBand(0, 440, 6, 1);
		eq.setPreamp(-6);
		check(eq.bandCount() == 1 && eq.bandGain(0) == 6, "equalizer has its band");
		var filtered = eq.process(audio);
		check(filtered.numberOfFrames() == frames, "filtered keeps its frames");

		var planes = new AudioDataCopyToOptions(Int64.ofInt(0));
		var samples = Bytes.alloc(Int64.toInt(filtered.allocationSize(planes)));
		filtered.copyTo(samples, planes);
		planes.close();
		var changed = false;
		for (i in 0...frames)
			if (samples.getFloat(i * 4) != pcm.getFloat(i * 4))
				changed = true;
		check(changed, "equalizer changed the samples");

		filtered.close();
		audio.close();
		eq.close();
		#if (wasm || emscripten)
		DecoderCheck.start();
		#end
	}
}
