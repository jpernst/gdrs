def can_build(platform):
	return True



def configure(env):
	if env["platform"] == "windows":
		raise AssertionError("TODO: windows support")
	else:
		env.Append(LINKFLAGS = "-rdynamic")
