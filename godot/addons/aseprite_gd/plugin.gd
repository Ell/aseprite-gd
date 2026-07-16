@tool
extends EditorPlugin
# Bootstraps the Rust importers plus the editor UI. A GDExtension
# EditorPlugin would auto-instantiate during the editor's first project scan,
# which segfaults Godot 4.7 headless; addon plugins load after the scan, so
# this thin GDScript shim wires everything instead.

var _importers: Array = []
var _inspector: EditorInspectorPlugin
var _preview: Control


func _enter_tree() -> void:
    for cls in ["AseTextureImporter", "AseSpriteFramesImporter", "AseAnimationLibraryImporter", "AseTilesetImporter", "AseStyleBoxImporter", "AseCanvasTextureImporter", "AseSceneImporter"]:
        if ClassDB.can_instantiate(cls):
            var importer = ClassDB.instantiate(cls)
            add_import_plugin(importer)
            _importers.append(importer)
        else:
            push_error("aseprite-gd: extension class %s unavailable (library not loaded?)" % cls)

    _inspector = preload("res://addons/aseprite_gd/inspector_plugin.gd").new()
    add_inspector_plugin(_inspector)
    _preview = preload("res://addons/aseprite_gd/preview_dock.gd").new()
    add_control_to_bottom_panel(_preview, "Aseprite")
    add_tool_menu_item("aseprite-gd: Reimport all Aseprite files", _reimport_all)


func _exit_tree() -> void:
    remove_tool_menu_item("aseprite-gd: Reimport all Aseprite files")
    if _preview != null:
        remove_control_from_bottom_panel(_preview)
        _preview.queue_free()
    if _inspector != null:
        remove_inspector_plugin(_inspector)
    for importer in _importers:
        remove_import_plugin(importer)
    _importers.clear()


func _reimport_all() -> void:
    var paths: Array = []
    _scan(EditorInterface.get_resource_filesystem().get_filesystem(), paths)
    if paths.is_empty():
        print("aseprite-gd: no aseprite files found")
        return
    EditorInterface.get_resource_filesystem().reimport_files(PackedStringArray(paths))
    print("aseprite-gd: reimported %d files" % paths.size())


func _scan(dir: EditorFileSystemDirectory, out: Array) -> void:
    for i in dir.get_file_count():
        var p := dir.get_file_path(i)
        if p.ends_with(".aseprite") or p.ends_with(".ase"):
            out.append(p)
    for i in dir.get_subdir_count():
        _scan(dir.get_subdir(i), out)
