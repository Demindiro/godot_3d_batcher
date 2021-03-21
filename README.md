# Godot 3D batcher

This addon will automatically batch nodes with the same mesh with MultiMeshes.
This can greatly reduce the amount of required drawcalls, improving performance.

## How to use

- Copy `addons/3d_batcher` to your project
- Add `BatchedMeshManager` as an Autoload
- Add a `BatchedMeshInstance` (or create a Spatial and add the
  `batched_mesh_instance.gdns` script manually)
- Add any mesh
- Done!

To use colors, set `use_color` to true and make sure the material uses vertex
colors as albedo (`COLOR` property in shader code).

## Limitations

Frustum culling does not work properly with shadows. I'm unsure how to fix
that properly. To disable it, add a script with
`BatchedMeshManager.enable_culling = false`.
