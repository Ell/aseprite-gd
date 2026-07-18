--- Aseprite previewer: renders .aseprite/.ase files with the `ase` CLI
--- (from the aseprite-gd repository: cargo install --path crates/ase-cli).
--- Scrolling the preview pane steps through the file's frames.

local M = {}

-- Frame counts per file, cached for the session so seeking can clamp.
local frame_counts = {}

local function frame_count(path)
	if frame_counts[path] then
		return frame_counts[path]
	end
	local output = Command("ase"):arg({ "info", path }):stdout(Command.PIPED):output()
	local n = 1
	if output and output.status.success then
		n = tonumber(output.stdout:match("frames:%s*(%d+)")) or 1
	end
	frame_counts[path] = n
	return n
end

function M:peek(job)
	local cache = ya.file_cache(job)
	if not cache then
		return
	end
	local ok, err = self:preload(job)
	if not ok then
		return ya.preview_widget(job, err)
	end
	ya.image_show(cache, job.area)
end

function M:seek(job)
	local h = cx.active.current.hovered
	if h and h.url == job.file.url then
		ya.emit("peek", {
			math.max(0, cx.active.preview.skip + job.units),
			only_if = job.file.url,
		})
	end
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
	local frame = math.min(job.skip or 0, frame_count(path) - 1)
	local output = Command("ase")
		:arg({ "render", path, tostring(frame), tostring(cache) })
		:stderr(Command.PIPED)
		:output()
	if not output then
		return false, "`ase` not found - install it from the aseprite-gd repo: cargo install --path crates/ase-cli"
	elseif not output.status.success then
		return false, "ase render failed: " .. (output.stderr or "unknown error")
	end
	return true
end

return M
