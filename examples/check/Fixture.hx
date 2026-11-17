#if macro
import haxe.macro.Context;
import haxe.macro.Expr;
#end

class Fixture {
	public static macro function tone():haxe.macro.Expr {
		return macro $v{sys.io.File.getContent(Context.resolvePath("tone-aac.json"))};
	}
}
