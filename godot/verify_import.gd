extends SceneTree

func _init():
    var tex = load("res://sprites/blend_multiply.aseprite")
    print("texture: ", tex, " size=", tex.get_size() if tex is Texture2D else "N/A")

    var sf = load("res://sprites/tags3.aseprite")
    print("sprite_frames: ", sf)
    if sf is SpriteFrames:
        for anim in sf.get_animation_names():
            var durs = []
            for i in sf.get_frame_count(anim):
                durs.append(sf.get_frame_duration(anim, i))
            print("  anim '", anim, "' frames=", sf.get_frame_count(anim), " loop=", sf.get_animation_loop(anim), " fps=", sf.get_animation_speed(anim), " durations=", durs)
    # Runtime API: parse + render without the import pipeline.
    var doc = AseDocument.open("res://sprites/tags3.aseprite")
    var doc_ok = false
    if doc != null:
        var img = doc.render_frame(0)
        doc_ok = doc.get_frame_count() == 12 \
            and doc.get_tag_names().size() == 3 \
            and doc.get_tag_range("pingpong") != Vector2i(-1, -1) \
            and img != null and img.get_size() == Vector2i(doc.get_size())
        print("ase_document: frames=", doc.get_frame_count(), " tags=", doc.get_tag_names(), " ok=", doc_ok)

    # SpriteFrames frames come from a shared packed atlas; margins restore
    # canvas size after trimming.
    var f0 = sf.get_frame_texture("forward", 0)
    var atlas_ok = f0 is AtlasTexture and f0.get_size() == Vector2(doc.get_size())
    print("atlas: ", f0, " size=", f0.get_size() if f0 != null else "N/A", " ok=", atlas_ok)

    # AnimationLibrary with texture value track + method track from cel user data.
    var lib = load("res://sprites/user_data.aseprite")
    var lib_ok = false
    if lib is AnimationLibrary:
        var names = lib.get_animation_list()
        var a = lib.get_animation(names[0])
        var has_tex_track = false
        var method_key = ""
        for ti in a.get_track_count():
            if a.track_get_type(ti) == Animation.TYPE_VALUE and String(a.track_get_path(ti)).ends_with(":texture"):
                has_tex_track = a.track_get_key_count(ti) > 0
            if a.track_get_type(ti) == Animation.TYPE_METHOD and a.track_get_key_count(ti) > 0:
                method_key = a.method_track_get_name(ti, 0)
        lib_ok = names.size() == 3 and has_tex_track and method_key == "test_user_data_cel"
        print("animlib: anims=", names, " tex_track=", has_tex_track, " method=", method_key, " ok=", lib_ok)

    # TileSet import: atlas source with tiles, source id = aseprite tileset id.
    var tset = load("res://sprites/tileset_sample.aseprite")
    var tset_ok = false
    if tset is TileSet:
        var src_count = tset.get_source_count()
        if src_count > 0:
            var src = tset.get_source(tset.get_source_id(0))
            tset_ok = src is TileSetAtlasSource and src.get_tiles_count() > 0 \
                and tset.tile_size == Vector2i(2, 2)
            print("tileset: sources=", src_count, " tiles=", src.get_tiles_count(), " tile_size=", tset.tile_size, " ok=", tset_ok)

    var ok = tex is Texture2D and sf is SpriteFrames and sf.get_animation_names().size() == 3 and doc_ok and atlas_ok and lib_ok and tset_ok
    print("VERIFY: ", "PASS" if ok else "FAIL")
    quit(0 if ok else 1)
