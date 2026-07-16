@tool
extends RefCounted
# Resource hook: stamp the file's tag names onto the imported resource.


func _post_import(resource: Resource, doc: AseDocument, _options: Dictionary, _source_file: String) -> Resource:
    resource.set_meta("ase_tags", doc.get_tag_names())
    return resource
