def can_build(platform):
	return True



def configure(env):
	if env["platform"] == "windows":
		raise AssertionError("TODO: link rust stdlib on windows")
	else:
		env.Append(LINKFLAGS = "-rdynamic")
