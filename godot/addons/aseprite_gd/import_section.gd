@tool
extends VBoxContainer
# Inspector section for AnimationPlayer / AnimatedSprite2D / AnimatedSprite3D:
# pick an .aseprite file and import into the selected node. Thin veneer over
# AseAnimationImport; all logic lives in the extension.

var _node: Object
var _path_edit: LineEdit
var _sprite_path_edit: LineEdit
var _split: CheckBox
var _slices: CheckBox
var _reset: CheckBox
var _status: Label
var _dialog: EditorFileDialog


func _init(node: Object) -> void:
    _node = node
    add_theme_constant_override("separation", 4)

    var title := Label.new()
    title.text = "Aseprite Import"
    title.add_theme_font_size_override("font_size", 14)
    add_child(title)

    var file_row := HBoxContainer.new()
    _path_edit = LineEdit.new()
    _path_edit.placeholder_text = "res://sprite.aseprite"
    _path_edit.size_flags_horizontal = Control.SIZE_EXPAND_FILL
    file_row.add_child(_path_edit)
    var browse := Button.new()
    browse.text = "..."
    browse.pressed.connect(_on_browse)
    file_row.add_child(browse)
    add_child(file_row)

    if _node is AnimationPlayer:
        var sp_row := HBoxContainer.new()
        var sp_label := Label.new()
        sp_label.text = "Sprite path"
        sp_row.add_child(sp_label)
        _sprite_path_edit = LineEdit.new()
        _sprite_path_edit.text = "Sprite2D"
        _sprite_path_edit.size_flags_horizontal = Control.SIZE_EXPAND_FILL
        sp_row.add_child(_sprite_path_edit)
        add_child(sp_row)

    _split = CheckBox.new()
    _split.text = "Split layers"
    add_child(_split)

    if _node is AnimationPlayer:
        _slices = CheckBox.new()
        _slices.text = "Slice hitbox tracks"
        add_child(_slices)
        _reset = CheckBox.new()
        _reset.text = "Create RESET animation"
        add_child(_reset)

    var import_row := HBoxContainer.new()
    var import_btn := Button.new()
    import_btn.text = "Import"
    import_btn.pressed.connect(_on_import)
    import_row.add_child(import_btn)
    if _node.has_meta("aseprite_gd_import"):
        var reimport_btn := Button.new()
        reimport_btn.text = "Reimport last"
        reimport_btn.pressed.connect(_on_reimport)
        import_row.add_child(reimport_btn)
    add_child(import_row)

    _status = Label.new()
    _status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
    add_child(_status)

    if _node.has_meta("aseprite_gd_import"):
        var meta = _node.get_meta("aseprite_gd_import")
        _path_edit.text = str(meta.get("file", ""))
        var opts = meta.get("options", {})
        _split.button_pressed = bool(opts.get("split_layers", false))
        if _node is AnimationPlayer:
            _sprite_path_edit.text = str(opts.get("sprite_path", "Sprite2D"))
            _slices.button_pressed = bool(opts.get("slice_tracks", false))
            _reset.button_pressed = bool(opts.get("create_reset_animation", false))


func _options() -> Dictionary:
    var opts := {"split_layers": _split.button_pressed}
    if _node is AnimationPlayer:
        opts["sprite_path"] = _sprite_path_edit.text
        opts["slice_tracks"] = _slices.button_pressed
        opts["create_reset_animation"] = _reset.button_pressed
    return opts


func _on_import() -> void:
    var path := _path_edit.text.strip_edges()
    if path.is_empty():
        _status.text = "Pick an .aseprite file first."
        return
    if _node is AnimationPlayer:
        var n = AseAnimationImport.merge_into_player(_node, path, _options())
        _status.text = "Merged %d animations." % n if n > 0 else "Import failed — see Output."
    else:
        var done = AseAnimationImport.assign_sprite_frames(_node, path, _options())
        _status.text = "SpriteFrames assigned." if done else "Import failed — see Output."
    _mark_scene_dirty()


func _on_reimport() -> void:
    if _node is AnimationPlayer:
        var n = AseAnimationImport.reimport(_node)
        _status.text = "Merged %d animations." % n if n > 0 else "Reimport failed — see Output."
    _mark_scene_dirty()


func _on_browse() -> void:
    if _dialog == null:
        _dialog = EditorFileDialog.new()
        _dialog.file_mode = EditorFileDialog.FILE_MODE_OPEN_FILE
        _dialog.access = EditorFileDialog.ACCESS_RESOURCES
        _dialog.add_filter("*.aseprite,*.ase", "Aseprite files")
        _dialog.file_selected.connect(func(p): _path_edit.text = p)
        add_child(_dialog)
    _dialog.popup_centered_ratio(0.5)


func _mark_scene_dirty() -> void:
    if Engine.is_editor_hint():
        EditorInterface.mark_scene_as_unsaved()
