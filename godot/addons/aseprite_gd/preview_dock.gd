@tool
extends VBoxContainer
# Bottom-panel animation preview: open (or drop) an .aseprite file, pick a
# tag, watch it play with the file's real frame timings.

var _doc  # AseDocument
var _path := ""
var _frames: Array[Texture2D] = []
var _order: Array[int] = []
var _index := 0
var _elapsed := 0.0
var _playing := false

var _open_btn: Button
var _tag_select: OptionButton
var _play_btn: Button
var _view: TextureRect
var _info: Label
var _dialog: EditorFileDialog


func _init() -> void:
    name = "AsepritePreview"
    var bar := HBoxContainer.new()
    _open_btn = Button.new()
    _open_btn.text = "Open..."
    _open_btn.pressed.connect(_on_open)
    bar.add_child(_open_btn)
    _tag_select = OptionButton.new()
    _tag_select.item_selected.connect(func(_i): _rebuild_order())
    bar.add_child(_tag_select)
    _play_btn = Button.new()
    _play_btn.text = "Pause"
    _play_btn.pressed.connect(_toggle_play)
    bar.add_child(_play_btn)
    _info = Label.new()
    bar.add_child(_info)
    add_child(bar)

    _view = TextureRect.new()
    _view.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
    _view.texture_filter = CanvasItem.TEXTURE_FILTER_NEAREST
    _view.size_flags_vertical = Control.SIZE_EXPAND_FILL
    _view.custom_minimum_size = Vector2(0, 160)
    add_child(_view)
    set_process(false)


func load_file(path: String) -> void:
    _doc = AseDocument.open(path)
    _frames.clear()
    _tag_select.clear()
    if _doc == null:
        _info.text = "failed to open %s" % path
        return
    _path = path
    _frames.resize(_doc.get_frame_count())
    _tag_select.add_item("(all frames)")
    for tag in _doc.get_tag_names():
        _tag_select.add_item(tag)
    _info.text = "%s — %d frames" % [path.get_file(), _doc.get_frame_count()]
    _rebuild_order()
    _playing = true
    _play_btn.text = "Pause"
    set_process(true)


func _rebuild_order() -> void:
    _order.clear()
    if _doc == null:
        return
    if _tag_select.selected <= 0:
        for i in _doc.get_frame_count():
            _order.append(i)
    else:
        var tag: String = _tag_select.get_item_text(_tag_select.selected)
        var r: Vector2i = _doc.get_tag_range(tag)
        for i in range(r.x, r.y + 1):
            _order.append(i)
    _index = 0
    _elapsed = 0.0
    _show_frame()


func _texture_for(frame: int) -> Texture2D:
    if _frames[frame] == null:
        _frames[frame] = _doc.render_texture(frame)
    return _frames[frame]


func _show_frame() -> void:
    if _order.is_empty():
        return
    _view.texture = _texture_for(_order[_index])


func _process(delta: float) -> void:
    if not _playing or _doc == null or _order.is_empty():
        return
    _elapsed += delta * 1000.0
    var dur: float = float(_doc.get_frame_duration_ms(_order[_index]))
    while _elapsed >= dur and dur > 0.0:
        _elapsed -= dur
        _index = (_index + 1) % _order.size()
        dur = float(_doc.get_frame_duration_ms(_order[_index]))
    _show_frame()


func _toggle_play() -> void:
    _playing = not _playing
    _play_btn.text = "Pause" if _playing else "Play"


func _on_open() -> void:
    if _dialog == null:
        _dialog = EditorFileDialog.new()
        _dialog.file_mode = EditorFileDialog.FILE_MODE_OPEN_FILE
        _dialog.access = EditorFileDialog.ACCESS_RESOURCES
        _dialog.add_filter("*.aseprite,*.ase", "Aseprite files")
        _dialog.file_selected.connect(load_file)
        add_child(_dialog)
    _dialog.popup_centered_ratio(0.5)


func _can_drop_data(_pos: Vector2, data) -> bool:
    return typeof(data) == TYPE_DICTIONARY and data.get("type", "") == "files" \
        and data["files"].size() > 0 \
        and (data["files"][0].ends_with(".aseprite") or data["files"][0].ends_with(".ase"))


func _drop_data(_pos: Vector2, data) -> void:
    load_file(data["files"][0])
