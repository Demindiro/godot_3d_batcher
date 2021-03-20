extends Node


export var count := 10000
export var batched := false
export(Array, Mesh) var meshes


func _ready() -> void:
	if batched:
		for i in count:
			var b := BatchedMeshInstance.new()
			b.mesh = meshes[i % len(meshes)]
			add_child(b)
	else:
		for i in count:
			var b := MeshInstance.new()
			b.mesh = meshes[i % len(meshes)]
			add_child(b)
		
