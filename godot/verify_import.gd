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

    var ok = tex is Texture2D and sf is SpriteFrames and sf.get_animation_names().size() == 3 and doc_ok
    print("VERIFY: ", "PASS" if ok else "FAIL")
    quit(0 if ok else 1)
