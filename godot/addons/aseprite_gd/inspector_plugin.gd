@tool
extends EditorInspectorPlugin

const ImportSection := preload("res://addons/aseprite_gd/import_section.gd")


func _can_handle(object: Object) -> bool:
    return object is AnimationPlayer or object is AnimatedSprite2D or object is AnimatedSprite3D


func _parse_begin(object: Object) -> void:
    add_custom_control(ImportSection.new(object))
