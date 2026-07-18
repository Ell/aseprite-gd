-- Tile naming for Aseprite: sets per-tile user data (the field the Godot
-- extension reads into the "aseprite_text" custom data layer / named-tile
-- extraction). Aseprite 1.3 has no built-in UI for this.
--
-- Install: File > Scripts > Open Scripts Folder, drop this file in, then
-- File > Scripts > Rescan Scripts Folder. Run with a tilemap layer active,
-- then click tiles in the Tileset panel to name them.

local spr = app.sprite
if not spr then
  return app.alert("Open a sprite first")
end
local lay = app.layer
if not lay or not lay.isTilemap then
  return app.alert("Select a tilemap layer first")
end
local ts = lay.tileset

-- The tile currently clicked in the Tileset panel (foreground tile).
local function current_index()
  local i = app.fgTile or 0
  if i < 1 or i >= #ts then
    return nil -- 0 is the empty tile; out of range = nothing usable selected
  end
  return i
end

local dlg = Dialog("Name tile")
local shown = -1

local function refresh()
  local i = current_index()
  if i == nil then
    dlg:modify{ id = "idx", text = "select a tile in the Tileset panel" }
    dlg:modify{ id = "name", text = "", enabled = false }
    shown = -1
  else
    dlg:modify{ id = "idx", text = "tile " .. i .. " / " .. (#ts - 1) }
    dlg:modify{ id = "name", text = ts:tile(i).data or "", enabled = true }
    shown = i
  end
  dlg:repaint()
end

local function save()
  if shown < 1 then
    return
  end
  local name = dlg.data.name or ""
  if ts:tile(shown).data ~= name then
    app.transaction("Name tile", function()
      ts:tile(shown).data = name
    end)
  end
end

dlg:canvas{
  id = "preview",
  width = 112,
  height = 112,
  onpaint = function(ev)
    if shown < 1 then
      return
    end
    local img = Image(ts:tile(shown).image)
    ev.context:drawImage(
      img,
      Rectangle(0, 0, img.width, img.height),
      Rectangle(8, 8, 96, 96)
    )
  end,
}
dlg:label{ id = "idx", text = "" }
dlg:entry{ id = "name", text = "" }
dlg:button{ text = "Save", focus = true, onclick = save }

-- Re-sync whenever the selected tile changes in the editor.
local listener = app.events:on("sitechange", function()
  local i = current_index()
  if i ~= shown then
    save() -- keep edits to the previously shown tile
    refresh()
  end
end)

dlg:show{
  wait = false,
  onclose = function()
    save()
    app.events:off(listener)
  end,
}
refresh()
