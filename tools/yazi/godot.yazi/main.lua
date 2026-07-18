--- Godot resource previewer: renders .tres/.res files to an image with a
--- headless Godot one-shot (preview.gd next to this file). Textures show as
--- themselves (AtlasTextures cropped to their region), TileSets show their
--- first atlas sheet, SpriteFrames their first frame. Non-visual .tres fall
--- back to the code previewer.

local M = {}

local function script_path()
	local config = os.getenv("XDG_CONFIG_HOME") or (os.getenv("HOME") .. "/.config")
	return config .. "/yazi/plugins/godot.yazi/preview.gd"
end

-- Nearest ancestor directory containing project.godot, so res:// references
-- inside the resource resolve.
local function project_root(path)
	local dir = path:match("(.*)/[^/]+$")
	while dir and dir ~= "" do
		local f = io.open(dir .. "/project.godot", "r")
		if f then
			f:close()
			return dir
		end
		dir = dir:match("(.*)/[^/]+$")
	end
	return nil
end

function M:peek(job)
	local cache = ya.file_cache(job)
	if not cache then
		return
	end
	local ok, err = self:preload(job)
	if not ok then
		if tostring(job.file.url):match("%.tres$") then
			return require("code"):peek(job) -- text resource: show the source
		end
		return ya.preview_widget(job, err)
	end
	ya.image_show(cache, job.area)
end

function M:preload(job)
	local cache = ya.file_cache(job)
	if not cache then
		return true
	end
	local cha = fs.cha(cache)
	if cha and cha.len > 0 then
		return true
	end

	local path = tostring(job.file.url)
	local args = { "--headless" }
	local root = project_root(path)
	if root then
		args[#args + 1] = "--path"
		args[#args + 1] = root
	end
	for _, a in ipairs({ "-s", script_path(), "--", path, tostring(cache) }) do
		args[#args + 1] = a
	end
	local output = Command("godot"):arg(args):stdout(Command.PIPED):stderr(Command.PIPED):output()
	if not output then
		return false, "`godot` not found in PATH"
	elseif not output.status.success then
		return false, "no previewable image in this resource"
	end
	local cha2 = fs.cha(cache)
	if not cha2 or cha2.len == 0 then
		return false, "no previewable image in this resource"
	end
	return true
end

return M
