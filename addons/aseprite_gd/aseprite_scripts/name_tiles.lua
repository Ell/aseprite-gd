-- Tile naming for Aseprite: sets per-tile user data (the field the Godot
-- extension reads into the "aseprite_text" custom data layer / named-tile
-- extraction). Aseprite 1.3 has no built-in UI for this.
--
-- Install: File > Scripts > Open Scripts Folder, drop this file in, then
-- File > Scripts > Rescan Scripts Folder. Run with a tilemap layer active.
-- Workflow: click a tile in the Tileset panel, press "Load selected" (or
-- Prev/Next), type a name, press Save.

local spr = app.sprite
if not spr then
  return app.alert("Open a sprite first")
end
local lay = app.layer
if not lay or not lay.isTilemap then
  return app.alert("Select a tilemap layer first")
end
local ts = lay.tileset

local shown = 1 -- tile 0 is the empty tile
local dlg = Dialog("Name tile")

local function clamp(i)
  return math.min(math.max(1, i), #ts - 1)
end

local function refresh()
  dlg:modify{ id = "idx", text = "tile " .. shown .. " / " .. (#ts - 1) }
  dlg:modify{ id = "name", text = ts:tile(shown).data or "" }
  dlg:repaint()
end

local function save()
  local name = dlg.data.name or ""
  if ts:tile(shown).data ~= name then
    app.transaction("Name tile", function()
      ts:tile(shown).data = name
    end)
  end
end

local function load_selected()
  local i = app.fgTile or 0
  if i < 1 or i >= #ts then
    return app.alert("Click a tile in the Tileset panel first")
  end
  save() -- keep the current edit before switching
  shown = i
  refresh()
end

local function step(delta)
  save()
  shown = clamp(shown + delta)
  refresh()
end

dlg:canvas{
  id = "preview",
  width = 112,
  height = 112,
  onpaint = function(ev)
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
dlg:newrow()
dlg:button{ text = "Load selected", onclick = load_selected }
dlg:button{ text = "< Prev", onclick = function() step(-1) end }
dlg:button{ text = "Next >", onclick = function() step(1) end }
dlg:newrow()
dlg:button{ text = "Save", focus = true, onclick = save }

dlg:show{ wait = false, onclose = save }
refresh()
