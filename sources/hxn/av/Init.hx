package hxn.av;

#if macro
import haxe.io.Path;
import haxe.macro.Compiler;
import haxe.macro.Context;
import sys.FileSystem;
import sys.io.File;
#end

/** Installs the browser decoder agent beside every wasm program. */
class Init {
	public static macro function register():haxe.macro.Expr {
		if (Context.defined("android")) {
			final api = Context.definedValue("HXCPP_ANDROID_PLATFORM");
			if (api == null || Std.parseInt(api) < 28)
				Compiler.define("HXCPP_ANDROID_PLATFORM", "28");
		}
		if (Context.defined("cpp") && (Context.defined("iphonesim") || Context.defined("iphoneos"))) {
			final linked = Context.definedValue("XAVI_LINK");
			if (linked != null) {
				final document = Xml.parse(File.getContent(linked));
				for (target in document.firstElement().elements())
					for (entry in target.elements()) {
						if (entry.nodeName == "lib" && StringTools.startsWith(entry.get("name"), "-l")) {
							final name = entry.get("name");
							entry.nodeName = "flag";
							entry.remove("name");
							entry.set("value", "-ref" + name);
						} else if (entry.nodeName == "flag" && entry.get("value") == "-framework")
							entry.set("value", "-ref-framework");
					}
				final out = FileSystem.absolutePath(Compiler.getOutput());
				FileSystem.createDirectory(out);
				final path = Path.join([out, "hxn_av_link.xml"]);
				File.saveContent(path, document.toString());
				Compiler.define("XAVI_LINK", path);
			}
		}
		if (Context.defined("emscripten") || Context.defined("wasm")) {
			var root = Context.resolvePath("hxn/av/Init.hx");
			for (_ in 0...4)
				root = Path.directory(root);
			Context.onAfterGenerate(() -> {
				final out = Compiler.getOutput();
				final directory = FileSystem.absolutePath(Context.defined("cpp") ? out : Path.directory(out));
				FileSystem.createDirectory(directory);
				File.copy(Path.join([root, "page", "hxn_av_decoder.mjs"]), Path.join([directory, "hxn_av_decoder.mjs"]));
			});
		}
		return macro null;
	}
}
