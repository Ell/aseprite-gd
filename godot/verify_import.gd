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
        var a = lib.get_animation("Tag 0")
        var has_tex_track = false
        var method_key = ""
        for ti in a.get_track_count():
            if a.track_get_type(ti) == Animation.TYPE_VALUE and String(a.track_get_path(ti)).ends_with(":texture"):
                has_tex_track = a.track_get_key_count(ti) > 0
            if a.track_get_type(ti) == Animation.TYPE_METHOD and a.track_get_key_count(ti) > 0:
                method_key = a.method_track_get_name(ti, 0)
        lib_ok = names.size() == 4 and has_tex_track and method_key == "test_user_data_cel"
        print("animlib: anims=", names, " tex_track=", has_tex_track, " method=", method_key, " ok=", lib_ok)

    # TileSet import: atlas source with tiles, source id = aseprite tileset id.
    var tset = load("res://sprites/tileset_sample.aseprite")
    var tset_ok = false
    if tset is TileSet:
        var src_count = tset.get_source_count()
        if src_count > 0:
            var src = tset.get_source(tset.get_source_id(0))
            tset_ok = src is TileSetAtlasSource and src.get_tiles_count() > 0 \
                and tset.tile_size == Vector2i(2, 2) \
                and src.resource_name == "Tileset (0)"
            print("tileset: sources=", src_count, " tiles=", src.get_tiles_count(), " tile_size=", tset.tile_size, " ok=", tset_ok)

    # 9-patch slice -> StyleBoxTexture with margins from the center rect.
    var sb = load("res://sprites/slices.aseprite")
    var sb_ok = false
    if sb is StyleBoxTexture:
        sb_ok = sb.texture != null and sb.texture.get_size() == Vector2(24, 16) \
            and sb.get_texture_margin(SIDE_LEFT) == 8.0 \
            and sb.get_texture_margin(SIDE_TOP) == 8.0 \
            and sb.get_texture_margin(SIDE_RIGHT) == 8.0 \
            and sb.get_texture_margin(SIDE_BOTTOM) == 4.0
        print("stylebox: tex=", sb.texture.get_size() if sb.texture else null, " margins=", [sb.get_texture_margin(SIDE_LEFT), sb.get_texture_margin(SIDE_TOP), sb.get_texture_margin(SIDE_RIGHT), sb.get_texture_margin(SIDE_BOTTOM)], " ok=", sb_ok)

    # Per-tile user data -> "aseprite_text" custom data layer.
    var tset2 = load("res://sprites/tile_flips.aseprite")
    var custom_ok = false
    if tset2 is TileSet and tset2.get_custom_data_layers_count() == 1:
        var src2 = tset2.get_source(tset2.get_source_id(0))
        var td = src2.get_tile_data(Vector2i(0, 0), 0)
        custom_ok = tset2.get_custom_data_layer_name(0) == "aseprite_text" \
            and td != null and td.get_custom_data("aseprite_text") == "solid"
        print("tile_custom_data: ", td.get_custom_data("aseprite_text") if td else null, " ok=", custom_ok)

    # Runtime ResourceFormatLoader: plain load() of a raw .aseprite with no
    # import pipeline involvement (game mode only).
    var raw = FileAccess.get_file_as_bytes("res://sprites/blend_multiply.aseprite")
    var fa = FileAccess.open("user://rt_test.aseprite", FileAccess.WRITE)
    fa.store_buffer(raw)
    fa.close()
    var rt = load("user://rt_test.aseprite")
    var rt_ok = rt is ImageTexture and rt.get_size() == Vector2(16, 16)
    print("runtime_loader: ", rt, " ok=", rt_ok)

    # Lit sprite: "normal" layer -> CanvasTexture normal map, excluded from diffuse.
    var ctex = load("res://sprites/lit_sprite.aseprite")
    var ct_ok = false
    if ctex is CanvasTexture:
        ct_ok = ctex.diffuse_texture != null and ctex.normal_texture != null \
            and ctex.specular_texture == null \
            and ctex.normal_texture.get_image().get_pixel(8, 8) == Color(128 / 255.0, 128 / 255.0, 1.0)
        print("canvas_texture: diffuse=", ctex.diffuse_texture != null, " normal=", ctex.normal_texture != null, " ok=", ct_ok)

    # Slice metadata via the runtime API (hitboxes, pivots, 9-patch info).
    var sdoc = AseDocument.open("res://sprites/slices.aseprite")
    var slices_ok = false
    if sdoc != null:
        var slices = sdoc.get_slices(0)
        var panel = null
        for sl in slices:
            if sl["name"] == "panel":
                panel = sl
        slices_ok = slices.size() == 2 and panel != null \
            and panel["rect"] == Rect2i(4, 4, 24, 16) \
            and panel["center"] == Rect2i(8, 8, 8, 4) \
            and panel["pivot"] == Vector2i(2, 3) \
            and panel["text"] == "nine"
        print("slices_api: ", slices.size(), " entries ok=", slices_ok)

    # Reimport-safe sync: user-authored TileData survives a re-sync.
    var owned := TileSet.new()
    owned.add_physics_layer()
    var sync_ok = false
    if AseTilesetSync.sync(owned, "res://sprites/tile_flips.aseprite") == 1:
        var src0: TileSetAtlasSource = owned.get_source(owned.get_source_id(0))
        var td0 = src0.get_tile_data(Vector2i(0, 0), 0)
        td0.add_collision_polygon(0)
        td0.set_collision_polygon_points(0, 0, PackedVector2Array([Vector2(0, 0), Vector2(8, 0), Vector2(8, 8)]))
        # second sync must keep the polygon and the aseprite custom data
        AseTilesetSync.sync(owned, "res://sprites/tile_flips.aseprite")
        var td1 = src0.get_tile_data(Vector2i(0, 0), 0)
        sync_ok = td1.get_collision_polygons_count(0) == 1 \
            and td1.get_custom_data("aseprite_text") == "solid"
        print("tileset_sync: polygons=", td1.get_collision_polygons_count(0), " text=", td1.get_custom_data("aseprite_text"), " ok=", sync_ok)

    # Externalized sheet: sync_with_sheet writes the atlas texture as its own
    # file and references it; a second sync updates that resource in place
    # (same instance, same path) rather than minting a new one.
    var shared := TileSet.new()
    var shsheet_ok = false
    if AseTilesetSync.sync_with_sheet(shared, "res://sprites/tile_flips.aseprite", "user://shared_sheet.res") == 1:
        var ssrc: TileSetAtlasSource = shared.get_source(shared.get_source_id(0))
        var stex = ssrc.texture
        AseTilesetSync.sync_with_sheet(shared, "res://sprites/tile_flips.aseprite", "user://shared_sheet.res")
        shsheet_ok = stex is PortableCompressedTexture2D \
            and stex.resource_path == "user://shared_sheet.res" \
            and shared.get_source(shared.get_source_id(0)).texture == stex \
            and FileAccess.file_exists("user://shared_sheet.res")
        print("shared_sheet: path=", stex.resource_path if stex else null, " ok=", shsheet_ok)

    # Slice hitbox tracks: "<slice>:position"/":size" value tracks (opt-in).
    var slib = load("res://sprites/slices_anim.aseprite")
    var strk_ok = false
    if slib is AnimationLibrary:
        var sanim = slib.get_animation(slib.get_animation_list()[0])
        var found_pos = false
        for ti in sanim.get_track_count():
            if String(sanim.track_get_path(ti)) == "panel:position" and sanim.track_get_key_count(ti) > 0:
                found_pos = sanim.track_get_key_value(ti, 0) == Vector2(4, 4)
        strk_ok = found_pos
        print("slice_tracks: ok=", strk_ok)

    # RESET animation + slice-cropped texture options.
    var lib2 = load("res://sprites/user_data.aseprite")
    var reset_ok = lib2 is AnimationLibrary and lib2.has_animation("RESET") \
        and lib2.get_animation("RESET").get_track_count() == 1
    var stex = load("res://sprites/slices_tex.aseprite")
    var stex_ok = stex is PortableCompressedTexture2D and stex.get_size() == Vector2(48, 32)
    var opts_ok = reset_ok and stex_ok
    print("option_features: reset=", reset_ok, " slice_tex=", stex_ok)

    # Grid split: a single-frame sheet chops into indexable cell textures.
    var gsheet = load("res://sprites/grid_sheet.aseprite")
    var grid_ok = false
    if gsheet is SpriteFrames:
        var c0 = gsheet.get_frame_texture("default", 0)
        grid_ok = gsheet.get_frame_count("default") == 16 \
            and c0 is AtlasTexture and c0.get_size() == Vector2(8, 8)
        print("grid_split: cells=", gsheet.get_frame_count("default"), " cell0=", c0.get_size(), " ok=", grid_ok)

    # Grid split x tags: "<tag>_<cell>" animation sets (grid = directions,
    # tags = actions), honoring ping-pong order and per-tag looping.
    var ganim = load("res://sprites/grid_anim.aseprite")
    var ganim_ok = false
    if ganim is SpriteFrames:
        var names2 = ganim.get_animation_names()
        ganim_ok = names2.size() == 12 and ganim.has_animation("walk_0") \
            and ganim.get_frame_count("walk_0") == 6 \
            and ganim.get_animation_loop("walk_0") \
            and not ganim.get_animation_loop("blink_3") \
            and ganim.get_frame_duration("idle_0", 0) == 200.0
        print("grid_anim: anims=", names2.size(), " walk_0=", ganim.get_frame_count("walk_0"), " ok=", ganim_ok)

    # Named-region extraction: owned folders of AtlasTextures + shared sheet.
    var ex_tile = load("res://extracted_tiles/solid.tres")
    var ex_slice = load("res://extracted_slices/panel.tres")
    var extract_ok = ex_tile is AtlasTexture and ex_tile.get_size() == Vector2(8, 8) \
        and ex_tile.atlas != null \
        and ex_slice is AtlasTexture and ex_slice.get_size() == Vector2(48, 32) \
        and ResourceLoader.exists("res://extracted_slices/hitbox.tres")
    print("extraction: tile=", ex_tile.get_size() if ex_tile else null, " slice=", ex_slice.get_size() if ex_slice else null, " ok=", extract_ok)

    # Dual output: one file imports as SpriteFrames while a hook syncs its
    # tilesets into a TileSet resource on every reimport.
    var dual_sf = load("res://sprites/dual.aseprite")
    var dual_ts = load("res://dual_tiles.tres")
    var dual_ok = dual_sf is SpriteFrames and dual_ts is TileSet \
        and dual_ts.get_source_count() == 1 and dual_ts.get_source(0).get_tiles_count() > 0
    if dual_ok:
        # hook uses sync_with_sheet: the sheet must live in its own file,
        # referenced from the .tres rather than embedded in it
        var dtex = dual_ts.get_source(dual_ts.get_source_id(0)).texture
        dual_ok = dtex is PortableCompressedTexture2D \
            and dtex.resource_path == "res://dual_tiles.sheet.res" \
            and FileAccess.get_file_as_string("res://dual_tiles.tres").contains("dual_tiles.sheet.res")
    print("dual_output: ok=", dual_ok)

    # Split-by-layer: one animation per visible leaf layer, isolated pixels.
    var split = load("res://sprites/split_layers.aseprite")
    var split_ok = false
    if split is SpriteFrames:
        var names = split.get_animation_names()
        var solo = split.get_frame_texture("inner_addition/default", 0)
        var solo_img = solo.get_atlas().get_image().get_region(Rect2i(solo.get_region()))
        # base layer's opaque right edge must NOT appear in the isolated layer
        var isolated = solo_img.get_size().x <= 10
        split_ok = names.size() == 3 and "base/default" in names \
            and "inner_normal/default" in names and "inner_addition/default" in names \
            and isolated
        print("split_layers: anims=", names, " isolated=", isolated, " ok=", split_ok)

    # Post-import hooks: resource hook stamps metadata; scene hook builds
    # hitbox nodes from slices before the scene is packed.
    var hook_ok = sf.get_meta("ase_tags", PackedStringArray()).size() == 3
    var hscene = load("res://sprites/slices_scene.aseprite")
    var hooks_ok = false
    if hscene is PackedScene:
        var hroot = hscene.instantiate()
        var hit = hroot.get_node_or_null("Hitboxes")
        var panel_shape = hit.get_node_or_null("panel") if hit else null
        hooks_ok = hook_ok and hit is Area2D and panel_shape is CollisionShape2D \
            and panel_shape.shape.size == Vector2(24, 16) \
            and panel_shape.position == Vector2(4 + 12, 4 + 8) \
            and hroot.get_node_or_null("AnimatedSprite2D") != null \
            and panel_shape.get_meta("ase_text", "") == "nine"
        print("post_import_hooks: resource_meta=", hook_ok, " scene=", hooks_ok)
        hroot.free()


    var ok = tex is Texture2D and sf is SpriteFrames and sf.get_animation_names().size() == 3 and doc_ok and atlas_ok and lib_ok and tset_ok and sb_ok and custom_ok and rt_ok and ct_ok and slices_ok and sync_ok and shsheet_ok and strk_ok and hooks_ok and split_ok and opts_ok and dual_ok and grid_ok and ganim_ok and extract_ok
    # Non-destructive AnimationPlayer merge: hand-made tracks and animations
    # survive re-import; imported tracks update without duplicating.
    var player := AnimationPlayer.new()
    var merge_opts = {"sprite_path": "Sprite2D"}
    var n1 = AseAnimationImport.merge_into_player(player, "res://sprites/user_data.aseprite", merge_opts)
    var mlib = player.get_animation_library("")
    # user customizations: extra track on an imported anim + a hand-made anim
    var tag0: Animation = mlib.get_animation("Tag 0")
    var custom_track = tag0.add_track(Animation.TYPE_VALUE)
    tag0.track_set_path(custom_track, "Sprite2D:modulate")
    tag0.track_insert_key(custom_track, 0.0, Color.RED)
    var hand := Animation.new()
    mlib.add_animation("hand_made", hand)
    var tracks_before = tag0.get_track_count()
    var n2 = AseAnimationImport.reimport(player)
    var tag0b: Animation = mlib.get_animation("Tag 0")
    var has_custom = false
    for ti in tag0b.get_track_count():
        if String(tag0b.track_get_path(ti)) == "Sprite2D:modulate":
            has_custom = true
    var merge_ok = n1 == 3 and n2 == 3 and has_custom \
        and tag0b.get_track_count() == tracks_before \
        and mlib.has_animation("hand_made")
    print("anim_merge: n1=", n1, " n2=", n2, " custom_kept=", has_custom, " tracks=", tag0b.get_track_count(), "/", tracks_before, " ok=", merge_ok)
    player.free()

    # Demo character: the AnimationPlayer showcase file must carry all the
    # merge inputs (tags, footstep user data, slice, two layers).
    var demo := AnimationPlayer.new()
    var dn = AseAnimationImport.merge_into_player(demo, "res://sprites/demo_character.aseprite", {"sprite_path": "Sprite2D", "slice_tracks": true, "create_reset_animation": true})
    var dlib = demo.get_animation_library("")
    var walk: Animation = dlib.get_animation("walk")
    var footsteps = 0
    var hitbox_track = false
    for ti in walk.get_track_count():
        if walk.track_get_type(ti) == Animation.TYPE_METHOD:
            footsteps = walk.track_get_key_count(ti)
        if String(walk.track_get_path(ti)) == "hurtbox:position":
            hitbox_track = true
    var demo_ok = dn == 4 and dlib.has_animation("idle") and dlib.has_animation("blink") \
        and dlib.has_animation("RESET") \
        and walk.loop_mode == Animation.LOOP_LINEAR \
        and dlib.get_animation("blink").loop_mode == Animation.LOOP_NONE \
        and footsteps == 3 and hitbox_track # ping-pong revisits the frame-3 footstep
    print("demo_character: anims=", dn, " footsteps=", footsteps, " hitbox=", hitbox_track, " ok=", demo_ok)
    demo.free()
    ok = ok and demo_ok

    # SpriteFrames assignment helper.
    var asprite := AnimatedSprite2D.new()
    var assign_ok = AseAnimationImport.assign_sprite_frames(asprite, "res://sprites/tags3.aseprite", {}) \
        and asprite.sprite_frames != null and asprite.sprite_frames.get_animation_names().size() == 3 \
        and asprite.has_meta("aseprite_gd_import")
    print("sprite_assign: ok=", assign_ok)
    asprite.free()
    ok = ok and merge_ok and assign_ok

    # Example scenes must instantiate with their imported resources wired up.
    var scene_ok = true
    for scene_path in ["res://examples/animated_character.tscn", "res://examples/ui_panel.tscn", "res://examples/lit_sprite.tscn", "res://examples/animation_player.tscn"]:
        var ps = load(scene_path)
        var inst = ps.instantiate() if ps != null else null
        if inst == null:
            scene_ok = false
            print("example scene FAILED to load: ", scene_path)
        else:
            inst.free()
    print("example_scenes: ok=", scene_ok)
    ok = ok and scene_ok

    print("VERIFY: ", "PASS" if ok else "FAIL")
    quit(0 if ok else 1)
