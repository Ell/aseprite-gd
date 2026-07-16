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
    var ok = tex is Texture2D and sf is SpriteFrames and sf.get_animation_names().size() == 3
    print("VERIFY: ", "PASS" if ok else "FAIL")
    quit(0 if ok else 1)
