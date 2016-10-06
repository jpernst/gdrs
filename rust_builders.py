import os
from SCons.Script import *



def _cargo_emitter(target, source, env):
	import fnmatch

	if (len(source) != 1 or os.path.basename(source[0].abspath) != "Cargo.toml"):
		raise AssertionError("RustStaticLib: `source` must be `Cargo.toml`")
	if (len(target) != 1):
		raise AssertionError("RustStaticLib: only one `target` allowed")

	for root, dirnames, filenames in os.walk(os.path.dirname(source[0].abspath)):
		for filename in fnmatch.filter(filenames, '*.rs'):
			source.append(os.path.join(root, filename))
	target = [os.path.join(os.path.dirname(target[0].abspath), os.path.splitext(os.path.basename(target[0].abspath))[0] + env["LIBSUFFIX"])]

	return target, source



def _cargo_static_generator(source, target, env, for_signature):
	actions = []
	debug_or_release = ""

	if (env["target"] == "release" or env["target"] == "release_debug"):
		actions.append("cargo build --manifest-path={} --release".format(source[0].abspath))
		debug_or_release = "release"
	else:
		actions.append("cargo build --manifest-path={}".format(source[0].abspath))
		debug_or_release = "debug"

	actions.append(Copy("$TARGET", os.path.join(
		os.path.dirname(source[0].abspath),
		"target",
		debug_or_release,
		"lib{0}.a".format(os.path.basename(os.path.dirname(source[0].abspath))))))

	return actions



_RustStaticLib = Builder(
	emitter = _cargo_emitter,
	generator = _cargo_static_generator)



def add_rust_builders(env):
	env["BUILDERS"]["RustStaticLib"] = _RustStaticLib
