import os
import subprocess
import glob
import fnmatch
import shutil

from SCons.Script import *
import toml



def add_rust_builders(env):
	if "RUSTC_VERSION" in env:
		return

	env["RUSTC_VERSION"] = subprocess.check_output(["rustc", "-V"])



	def rustc_version(env):
		target = env._RustcVersion("rustc-version", [])
		AlwaysBuild(target)
		return target



	def write_rustc_version(target, source, env):
		with open(target[0].abspath, "wb") as dest:
			dest.write(env["RUSTC_VERSION"])



	def scan_cargo_toml(manifest_path, source):
		source.append(os.path.join(os.path.dirname(manifest_path), "Cargo.lock"))
		for root, dirnames, filenames in os.walk(os.path.dirname(manifest_path)):
			for filename in fnmatch.filter(filenames, '*.rs'):
				source.append(os.path.join(root, filename))

		manifest = toml.load(manifest_path)
		for section in ["dependencies", "build-dependencies"]:
			if section in manifest and isinstance(manifest[section], dict):
				for _, dep in manifest[section].iteritems():
					if isinstance(dep, dict) and "path" in dep:
						dep_manifest_path = os.path.join(os.path.dirname(manifest_path), dep["path"], "Cargo.toml")
						source.append(dep_manifest_path)
						scan_cargo_toml(dep_manifest_path, source)



	def rust_emitter(target, source, env):
		if len(source) != 2 or os.path.basename(source[1].abspath) != "Cargo.toml":
			raise AssertionError("cargo_emitter: `source` must be `Cargo.toml`")
		if len(target) != 1:
			raise AssertionError("cargo_emitter: only one `target` allowed")

		scan_cargo_toml(source[1].abspath, source)

		target = [os.path.join(
			os.path.dirname(target[0].abspath),
			os.path.splitext(os.path.basename(target[0].abspath))[0] + env["LIBSUFFIX"])]

		return target, source



	def rust_staticlib_generator(source, target, env, for_signature):
		if env["target"] == "release" or env["target"] == "release_debug":
			actions = ["cargo build -q --manifest-path={} --release".format(source[1].abspath)]
			profile = "release"
		else:
			actions = ["cargo build -q --manifest-path={}".format(source[1].abspath)]
			profile = "debug"

		actions.append(Copy("$TARGET", glob.glob(os.path.join( os.path.dirname(source[1].abspath), "target", profile, "lib*.a"))[0]))

		return actions



	def rust_godot_module_emitter(target, source, env):
		target, source = rust_emitter(target, source, env)

		target.append(os.path.join(os.path.dirname(target[0]), "godot_macros.cpp"))

		return target, source



	def concat_macros_cpp(target, source, env):
		with open(os.path.join(os.path.dirname(target[0].abspath), "godot_macros.cpp"), "wb") as dest:
			for filename in glob.iglob(os.path.join(env["ENV"]["GODOT_MACROS_CPP_DIR"], "*.cpp")):
				with open(filename, "rb") as src:
					shutil.copyfileobj(src, dest)



	def rust_godot_module_generator(source, target, env, for_signature):
		godot_macros_cpp_dir = os.path.join(os.path.dirname(source[-1].abspath), "godot_macros.cpp.d")
		env["ENV"]["GODOT_MACROS_CPP_DIR"] = godot_macros_cpp_dir

		return [
			Delete(godot_macros_cpp_dir)
		] + rust_staticlib_generator(source, target, env, for_signature) + [
			concat_macros_cpp
		]



	def rust_staticlib(env, target):
		return env._RustStaticLib(target, [env.RustcVersion(), "Cargo.toml"])



	def rust_godot_module(env, target):
		mod_env = env.Clone()
		return mod_env._RustGodotModule(target, [env.RustcVersion(), "Cargo.toml"])



	env["BUILDERS"]["_RustcVersion"] = Builder(
		action = write_rustc_version)
	env["BUILDERS"]["_RustStaticLib"] = Builder(
		emitter = rust_emitter,
		generator = rust_staticlib_generator)
	env["BUILDERS"]["_RustGodotModule"] = Builder(
		emitter = rust_godot_module_emitter,
		generator = rust_godot_module_generator)

	env.AddMethod(rustc_version, "RustcVersion")
	env.AddMethod(rust_staticlib, "RustStaticLib")
	env.AddMethod(rust_godot_module, "RustGodotModule")
