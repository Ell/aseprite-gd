-- Systematic corpus generator. Run via tools/corpus/generate.sh (headless):
--
--   aseprite -b --script-param out=DIR --script tools/corpus/gen_corpus.lua
--
-- Emits one .aseprite per case plus a golden .png flattened by Aseprite
-- itself — the reference implementation renders our expected output.
-- Fixtures are small (16x16) on purpose: pixel content exercises the
-- compositor, not storage volume.

local out = app.params["out"] or "corpus_out"

-- Base backdrop: alpha ramp (transparent / semi / opaque bands) under the
-- blended cel, so every blend mode is tested against alpha=0, partial and
-- full backdrop — the legacy-vs-new blend divergence lives in the non-opaque
-- bands (format ref §9.4).
local function base_image(w, h)
  local img = Image(w, h, ColorMode.RGB)
  for y = 0, h - 1 do
    for x = 0, w - 1 do
      local a
      if x < 5 then a = 0 elseif x < 10 then a = 128 else a = 255 end
      img:putPixel(x, y, app.pixelColor.rgba(x * 16, y * 16, 255 - x * 16, a))
    end
  end
  return img
end

-- Source layer: offset cel, varied colors, semi-transparent alpha ramp so
-- opacity multiplication paths (layer 200/255 x per-pixel alpha) are hit.
local function top_image(w, h)
  local img = Image(w, h, ColorMode.RGB)
  for y = 0, h - 1 do
    for x = 0, w - 1 do
      img:putPixel(x, y, app.pixelColor.rgba(255 - y * 20, x * 20, y * 20, 128 + x * 10))
    end
  end
  return img
end

local modes = {
  { "normal", BlendMode.NORMAL },
  { "multiply", BlendMode.MULTIPLY },
  { "screen", BlendMode.SCREEN },
  { "overlay", BlendMode.OVERLAY },
  { "darken", BlendMode.DARKEN },
  { "lighten", BlendMode.LIGHTEN },
  { "color_dodge", BlendMode.COLOR_DODGE },
  { "color_burn", BlendMode.COLOR_BURN },
  { "hard_light", BlendMode.HARD_LIGHT },
  { "soft_light", BlendMode.SOFT_LIGHT },
  { "difference", BlendMode.DIFFERENCE },
  { "exclusion", BlendMode.EXCLUSION },
  { "hsl_hue", BlendMode.HSL_HUE },
  { "hsl_saturation", BlendMode.HSL_SATURATION },
  { "hsl_color", BlendMode.HSL_COLOR },
  { "hsl_luminosity", BlendMode.HSL_LUMINOSITY },
  { "addition", BlendMode.ADDITION },
  { "subtract", BlendMode.SUBTRACT },
  { "divide", BlendMode.DIVIDE },
}

for _, m in ipairs(modes) do
  local name, mode = m[1], m[2]
  local spr = Sprite(16, 16, ColorMode.RGB)
  spr.layers[1].name = "base"
  spr.cels[1].image:drawImage(base_image(16, 16))

  local top = spr:newLayer()
  top.name = "top"
  top.blendMode = mode
  top.opacity = 200
  spr:newCel(top, 1, top_image(12, 12), Point(2, 2))

  spr:saveAs(out .. "/blend_" .. name .. ".aseprite")
  spr:saveCopyAs(out .. "/blend_" .. name .. ".png")
  spr:close()
end

-- Group blend/opacity: a group with its own blend mode + opacity containing
-- children with non-normal blends. Exercises group buffers (§6.2 NOTE.6) —
-- children must blend against the group's own backdrop, not the base layer.
do
  local spr = Sprite(16, 16, ColorMode.RGB)
  spr.layers[1].name = "base"
  spr.cels[1].image:drawImage(base_image(16, 16))

  local grp = spr:newGroup()
  grp.name = "fx"

  local inner1 = spr:newLayer()
  inner1.parent = grp
  inner1.name = "inner_normal"
  spr:newCel(inner1, 1, top_image(10, 10), Point(1, 1))

  local inner2 = spr:newLayer()
  inner2.parent = grp
  inner2.name = "inner_addition"
  inner2.blendMode = BlendMode.ADDITION
  inner2.opacity = 180
  spr:newCel(inner2, 1, top_image(10, 10), Point(5, 5))

  grp.blendMode = BlendMode.MULTIPLY
  grp.opacity = 160

  spr:saveAs(out .. "/group_blend.aseprite")
  spr:saveCopyAs(out .. "/group_blend.png")
  spr:close()
end

-- Cel z-index: bottom layer's cel hops above the top layer's (§8 NOTE.5).
do
  local spr = Sprite(16, 16, ColorMode.RGB)
  spr.layers[1].name = "low"
  spr.cels[1].image:drawImage(base_image(16, 16))

  local hi = spr:newLayer()
  hi.name = "high"
  spr:newCel(hi, 1, top_image(12, 12), Point(2, 2))

  spr.layers[1]:cel(1).zIndex = 2
  hi:cel(1).zIndex = -1

  spr:saveAs(out .. "/zindex.aseprite")
  spr:saveCopyAs(out .. "/zindex.png")
  spr:close()
end

-- Tilemap with flipped tiles: X/Y/D flip bits on tile references (§6.3
-- type 3). Tile 1 is asymmetric so every flip is visually distinct.
do
  local spr = Sprite(16, 16, ColorMode.RGB)
  spr.layers[1].name = "base"
  spr.cels[1].image:drawImage(base_image(16, 16))

  app.command.NewLayer { tilemap = true, gridBounds = Rectangle(0, 0, 8, 8) }
  local lay = spr.layers[#spr.layers]
  local ts = lay.tileset

  local tile = spr:newTile(ts)
  local timg = Image(8, 8, ColorMode.RGB)
  for y = 0, 7 do
    for x = 0, 7 do
      if x > y then
        timg:putPixel(x, y, app.pixelColor.rgba(255, x * 30, 0, 255))
      elseif x == y then
        timg:putPixel(x, y, app.pixelColor.rgba(0, 0, 0, 0))
      else
        timg:putPixel(x, y, app.pixelColor.rgba(0, y * 30, 255, 255))
      end
    end
  end
  tile.image = timg

  local grid = Image(2, 2, ColorMode.TILEMAP)
  grid:putPixel(0, 0, 1) -- plain
  grid:putPixel(1, 0, 1 | 0x80000000) -- x flip
  grid:putPixel(0, 1, 1 | 0x40000000) -- y flip
  grid:putPixel(1, 1, 1 | 0x20000000) -- d flip
  spr:newCel(lay, 1, grid, Point(0, 0))

  spr:saveAs(out .. "/tile_flips.aseprite")
  spr:saveCopyAs(out .. "/tile_flips.png")
  spr:close()
end

print("corpus written to " .. out)
