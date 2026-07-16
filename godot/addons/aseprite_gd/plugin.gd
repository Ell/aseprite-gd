@tool
extends EditorPlugin
# Bootstraps the Rust importers. A GDExtension EditorPlugin would auto-
# instantiate during the editor's first project scan, which segfaults
# Godot 4.7 headless; addon plugins load after the scan, so this thin
# GDScript shim instantiates the importer classes instead.

var _importers: Array = []


func _enter_tree() -> void:
    for cls in ["AseTextureImporter", "AseSpriteFramesImporter", "AseAnimationLibraryImporter", "AseTilesetImporter", "AseStyleBoxImporter", "AseCanvasTextureImporter", "AseSceneImporter"]:
        if ClassDB.can_instantiate(cls):
            var importer = ClassDB.instantiate(cls)
            add_import_plugin(importer)
            _importers.append(importer)
        else:
            push_error("aseprite-gd: extension class %s unavailable (library not loaded?)" % cls)


func _exit_tree() -> void:
    for importer in _importers:
        remove_import_plugin(importer)
    _importers.clear()
