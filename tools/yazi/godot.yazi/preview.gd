extends SceneTree
# Renders a Godot resource to a PNG for terminal previews (see main.lua).
# Usage: godot --headless [--path <project>] -s preview.gd -- <src> <out.png>


func _init() -> void:
	var args := OS.get_cmdline_user_args()
	if args.size() < 2:
		quit(2)
		return
	var res: Resource = ResourceLoader.load(args[0])
	if res == null:
		quit(1)
		return
	var img := _image_for(res)
	if img == null:
		quit(1)
		return
	if img.is_compressed():
		img.decompress()
	# Pixel art reads better upscaled; integer nearest keeps it crisp.
	var k := 1
	while maxi(img.get_width(), img.get_height()) * k < 512 and k < 16:
		k += 1
	if k > 1:
		img.resize(img.get_width() * k, img.get_height() * k, Image.INTERPOLATE_NEAREST)
	quit(0 if img.save_png(args[1]) == OK else 1)


func _image_for(res: Resource) -> Image:
	if res is AtlasTexture and res.atlas != null:
		var a: Image = res.atlas.get_image()
		if a != null:
			if a.is_compressed():
				a.decompress()
			return a.get_region(Rect2i(res.region))
	if res is Texture2D:
		return res.get_image()
	if res is CanvasTexture and res.diffuse_texture != null:
		return res.diffuse_texture.get_image()
	if res is StyleBoxTexture and res.texture != null:
		return res.texture.get_image()
	if res is TileSet:
		for i in res.get_source_count():
			var s := (res as TileSet).get_source(res.get_source_id(i))
			if s is TileSetAtlasSource and s.texture != null:
				return s.texture.get_image()
	if res is SpriteFrames:
		for anim in res.get_animation_names():
			if res.get_frame_count(anim) > 0:
				return res.get_frame_texture(anim, 0).get_image()
	return null
