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



	def write_rustc_version(target, source, env):
		with open(str(target[0]), "wb") as dest:
			dest.write(env["RUSTC_VERSION"])



	def scan_cargo_toml(node, env):
		source = []
		if os.path.basename(str(node)) == "Cargo.toml":
			scan_cargo_toml_impl(str(node), source)
		return source



	def scan_cargo_toml_impl(manifest_path, source):
		cargo_lock_path = os.path.join(os.path.dirname(manifest_path), "Cargo.lock")
		if os.path.exists(cargo_lock_path):
			source.append(File(cargo_lock_path))

		for root, dirnames, filenames in os.walk(os.path.dirname(manifest_path)):
			for filename in fnmatch.filter(filenames, '*.rs'):
				source.append(File(os.path.join(root, filename)))

		manifest = toml.load(manifest_path)
		for section in ["dependencies", "build-dependencies"]:
			if section in manifest and isinstance(manifest[section], dict):
				for _, dep in manifest[section].iteritems():
					if isinstance(dep, dict) and "path" in dep:
						dep_manifest_path = os.path.join(os.path.dirname(manifest_path), dep["path"], "Cargo.toml")
						source.append(File(dep_manifest_path))
						scan_cargo_toml_impl(dep_manifest_path, source)



	def rust_program_emitter(target, source, env):
		if len(source) != 2 or os.path.basename(str(source[0])) != "rustc-version" or os.path.basename(str(source[1])) != "Cargo.toml":
			raise AssertionError("rust_program_emitter: `source` must be [`rustc-version`, `Cargo.toml`]")
		if len(target) != 1:
			raise AssertionError("rust_program_emitter: only one `target` allowed")

		source.extend(scan_cargo_toml(source[1], env))
		target = [File(os.path.join(
			os.path.dirname(str(target[0])),
			os.path.splitext(os.path.basename(str(target[0])))[0] + env["PROGSUFFIX"]
		))]

		return target, source



	def rust_program_generator(source, target, env, for_signature):
		if env["target"] == "release" or env["target"] == "release_debug":
			actions = ["cargo build -q --manifest-path={} --release".format(str(source[1]))]
			profile = "release"
		else:
			actions = ["cargo build -q --manifest-path={}".format(str(source[1]))]
			profile = "debug"

		crate_name = toml.load(str(source[1]))["package"]["name"]

		actions.append(Copy("$TARGET", os.path.join(os.path.dirname(str(source[1])), "target", profile, crate_name)))

		return actions



	def rust_staticlib_emitter(target, source, env):
		if len(source) != 2 or os.path.basename(str(source[0])) != "rustc-version" or os.path.basename(str(source[1])) != "Cargo.toml":
			raise AssertionError("rust_staticlib_emitter: `source` must be [`rustc-version`, `Cargo.toml`]")
		if len(target) != 1:
			raise AssertionError("rust_staticlib_emitter: only one `target` allowed")

		source.extend(scan_cargo_toml(source[1], env))
		target = [os.path.join(
			os.path.dirname(str(target[0])),
			os.path.splitext(os.path.basename(str(target[0])))[0] + env["LIBSUFFIX"])]

		return target, source



	def rust_staticlib_generator(source, target, env, for_signature):
		if env["target"] == "release" or env["target"] == "release_debug":
			actions = ["cargo build -q --manifest-path={} --release".format(str(source[1]))]
			profile = "release"
		else:
			actions = ["cargo build -q --manifest-path={}".format(str(source[1]))]
			profile = "debug"

		crate_name = toml.load(str(source[1]))["package"]["name"].replace("-", "_")

		actions.append(Copy("$TARGET", os.path.join(os.path.dirname(str(source[1])), "target", profile, "lib{}.a".format(crate_name))))

		return actions



	def rust_godot_module_emitter(target, source, env):
		target, source = rust_staticlib_emitter(target, source, env)

		target.append(os.path.join(os.path.dirname(target[0]), "gdrs-macros.cpp"))

		return target, source



	def concat_macros_cpp(target, source, env):
		with open(os.path.join(os.path.dirname(str(target[0])), "gdrs-macros.cpp"), "wb") as dest:
			for filename in glob.iglob(os.path.join(env["ENV"]["GDRS_MACROS_CPP_DIR"], "*.cpp")):
				with open(filename, "rb") as src:
					shutil.copyfileobj(src, dest)



	def rust_godot_module_generator(source, target, env, for_signature):
		gdrs_macros_cpp_dir = os.path.join(os.path.dirname(str(source[-1])), "gdrs-macros.cpp.d")
		env["ENV"]["GDRS_MACROS_CPP_DIR"] = gdrs_macros_cpp_dir

		return [
			Delete(gdrs_macros_cpp_dir)
		] + rust_staticlib_generator(source, target, env, for_signature) + [
			concat_macros_cpp
		]



	env.Append(
		SCANNERS = [Scanner(scan_cargo_toml)],
		BUILDERS = {
			"_RustcVersion": Builder(action = write_rustc_version),
			"_RustProgram": Builder(emitter = rust_program_emitter, generator = rust_program_generator),
			"_RustStaticLib": Builder(emitter = rust_staticlib_emitter, generator = rust_staticlib_generator),
			"_RustGodotModule": Builder(emitter = rust_godot_module_emitter, generator = rust_godot_module_generator)
		}
	)

	def rustc_version(env):
		target = env._RustcVersion("rustc-version", [])
		AlwaysBuild(target)
		return target

	def rust_program(env, target, root = "."):
		return env._RustProgramLib(target, [env.RustcVersion(), os.path.normpath(os.path.join(root, "Cargo.toml"))])

	def rust_staticlib(env, target, root = "."):
		return env._RustStaticLib(target, [env.RustcVersion(), os.path.normpath(os.path.join(root, "Cargo.toml"))])

	def rust_godot_module(env, target, root = "."):
		mod_env = env.Clone()
		return mod_env._RustGodotModule(target, [env.RustcVersion(), os.path.normpath(os.path.join(root, "Cargo.toml"))])

	env.AddMethod(rustc_version, "RustcVersion")
	env.AddMethod(rust_program, "RustProgram")
	env.AddMethod(rust_staticlib, "RustStaticLib")
	env.AddMethod(rust_godot_module, "RustGodotModule")
