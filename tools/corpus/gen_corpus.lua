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

print("corpus written to " .. out)
