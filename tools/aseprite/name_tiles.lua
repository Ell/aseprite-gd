-- Tile naming dialog for Aseprite: sets per-tile user data (the field the
-- Godot extension reads into the "aseprite_text" custom data layer and the
-- named-tile extraction hooks). Aseprite 1.3 has no built-in UI for this.
--
-- Install: File > Scripts > Open Scripts Folder, drop this file in, then
-- File > Scripts > Rescan Scripts Folder. Run with a tilemap layer active.

local spr = app.sprite
if not spr then
  return app.alert("Open a sprite first")
end
local lay = app.layer
if not lay or not lay.isTilemap then
  return app.alert("Select a tilemap layer first")
end
local ts = lay.tileset
if #ts < 2 then
  return app.alert("This tileset has no tiles")
end

local index = 1 -- tile 0 is the empty tile
local dlg = Dialog("Tile names (" .. (ts.name ~= "" and ts.name or "tileset") .. ")")

local function commit()
  local name = dlg.data.name or ""
  if ts:tile(index).data ~= name then
    app.transaction("Name tile", function()
      ts:tile(index).data = name
    end)
  end
end

local function refresh()
  dlg:modify{ id = "idx", text = "tile " .. index .. " / " .. (#ts - 1) }
  dlg:modify{ id = "name", text = ts:tile(index).data or "" }
  dlg:repaint()
end

local function go(delta)
  commit()
  index = math.min(math.max(1, index + delta), #ts - 1)
  refresh()
end

dlg:canvas{
  id = "preview",
  width = 128,
  height = 128,
  onpaint = function(ev)
    local g = ev.context
    local img = Image(ts:tile(index).image)
    g:drawImage(
      img,
      Rectangle(0, 0, img.width, img.height),
      Rectangle(16, 16, 96, 96)
    )
  end,
}
dlg:label{ id = "idx", text = "tile 1 / " .. (#ts - 1) }
dlg:entry{ id = "name", text = ts:tile(1).data or "" }
dlg:button{ text = "< Prev", onclick = function() go(-1) end }
dlg:button{ text = "Next >", onclick = function() go(1) end }
dlg:button{
  text = "Close",
  onclick = function()
    commit()
    dlg:close()
  end,
}
dlg:show{ wait = false }
