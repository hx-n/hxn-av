import haxe.Int64;
import haxe.crypto.Base64;
import haxe.io.Bytes;
import hxn.av.*;

/** Polls browser decoding without waiting on the page thread. */
class DecoderCheck {
	static var decoder:AudioDecoder;
	static var packets:Array<Dynamic>;
	static var pending:EncodedAudioChunk;
	static var index = 0;
	static var frames = 0;
	static var ticks = 0;
	static var ending = false;
	static var finished = false;
	static var timer:haxe.Timer;
	static var retried = false;
	static var rate:Int;

	public static function start():Void {
		try {
			final fixture:Dynamic = haxe.Json.parse(Fixture.tone());
			rate = fixture.sampleRate;
			packets = fixture.packets;
			final config = CodecConfiguration.audio("mp4a.40.2", Int64.ofInt(rate), Int64.ofInt(fixture.channels), Base64.decode(fixture.description));
			decoder = AudioDecoder.create(2, Int64.ofInt(16384), config);
			config.close();
			#if emscripten
			hxn.native.Emscripten.later(tick);
			#else
			timer = new haxe.Timer(10);
			timer.run = tick;
			#end
		} catch (error:Dynamic)
			fail(error);
	}

	static function tick():Void {
		try {
			if (++ticks > 1000)
				throw "AAC decoder timed out";
			for (_ in 0...4) {
				if (index == packets.length)
					break;
				if (pending == null) {
					final packet = packets[index];
					final init = new EncodedAudioChunkInit(EncodedChunkType.Key, Int64.ofInt(packet.timestamp), Base64.decode(packet.data));
					pending = EncodedAudioChunk.create(init);
					init.close();
				}
				if (!decoder.tryDecode(pending)) {
					retried = true;
					break;
				}
				pending.close();
				pending = null;
				index++;
			}
			if (index == packets.length && !ending) {
				decoder.finish();
				ending = true;
			}
			for (_ in 0...4) {
				switch (decoder.poll()) {
					case StreamReadStatus.Pending:
						break;
					case StreamReadStatus.Ready:
						final data = decoder.read();
						final count = Int64.toInt(data.numberOfFrames());
						final options = new AudioDataCopyToOptions(Int64.ofInt(0));
						options.format(AudioSampleFormat.F32);
						final bytes = Bytes.alloc(Int64.toInt(data.allocationSize(options)));
						data.copyTo(bytes, options);
						options.close();
						for (i in 0...Std.int(bytes.length / 4))
							if (!Math.isFinite(bytes.getFloat(i * 4)))
								throw "nonfinite decoder output";
						frames += count;
						data.close();
					case StreamReadStatus.Ended:
						if (frames != 49152 || !retried)
							throw 'AAC frame count or backpressure failed: $frames';
						decoder.close();
						var closed = false;
						try
							decoder.poll()
						catch (_:Dynamic)
							closed = true;
						if (!closed)
							throw "closed decoder remained usable";
						finished = true;
						#if !emscripten
						timer.stop();
						#end
						Sys.println("AAC decode, backpressure, EOF and close: yes");
						Main.leave(0);
					case _:
						throw "invalid decoder status";
				}
				if (finished)
					break;
			}
		} catch (error:Dynamic)
			fail(error);
		#if emscripten
		if (!finished)
			hxn.native.Emscripten.later(tick);
		#end
	}

	static function fail(error:Dynamic):Void {
		finished = true;
		Sys.println('AAC decode failed: ${Std.string(error)}');
		Main.leave(1);
	}
}
