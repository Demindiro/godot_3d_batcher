use gdnative::api::{Mesh, Node, VisualServer};
use gdnative::prelude::*;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::RwLock;

const MULTIMESH_ALLOC_STEP: usize = 256;

lazy_static! {
	static ref MULTI_MESHES: RwLock<(
		usize,
		HashMap<Ref<Mesh>, (Rid, TypedArray<f32>, Vec<(usize, Ref<Spatial>)>, Rid)>,
	)> = RwLock::new((0, HashMap::new()));
}

#[derive(NativeClass)]
#[inherit(Node)]
struct BatchedMeshManager {}

#[derive(NativeClass)]
#[inherit(Spatial)]
struct BatchedMeshInstance {
	#[property(
		before_set = "Self::pre_change_mesh",
		after_set = "Self::post_change_mesh"
	)]
	mesh: Option<Ref<Mesh>>,
	id: Option<usize>,
}

#[methods]
impl BatchedMeshManager {
	fn new(_owner: TRef<Node>) -> Self {
		Self {}
	}

	#[export]
	fn _process(&self, owner: TRef<Node>, _delta: f32) {
		unsafe {
			owner.call_deferred("_update_transforms", &[]);
		}
	}

	#[export]
	#[profiled(tag = "Batcher/Update transforms")]
	fn _update_transforms(&self, _owner: TRef<Node>) {
		let mut map = MULTI_MESHES.write().expect("Failed to read MULTI_MESHES");
		let vs = unsafe { VisualServer::godot_singleton() };
		for (mm_rid, trfs, nodes, _) in map.1.values_mut() {
			let mut w = trfs.write();
			for (i, (_, n)) in nodes.iter().enumerate() {
				let trf = unsafe { n.assume_safe().global_transform() };
				for (k, &e) in transform_to_array(trf).iter().enumerate() {
					w[i * 12 + k] = e;
				}
			}
			drop(w);
			vs.multimesh_set_as_bulk_array(*mm_rid, trfs.clone());
		}
	}
}

#[methods]
impl BatchedMeshInstance {
	fn new(_owner: TRef<Spatial>) -> Self {
		Self {
			mesh: None,
			id: None,
		}
	}

	#[export]
	fn _enter_tree(&mut self, owner: TRef<Spatial>) {
		debug_assert_eq!(self.id, None);
		if let Some(mesh) = &self.mesh {
			let rid = { owner.get_world().unwrap() };
			let rid = unsafe { rid.assume_safe() };
			self.id = Some(add_instance(mesh.clone(), owner.claim(), rid));
		}
	}

	#[export]
	fn _exit_tree(&mut self, _owner: TRef<Spatial>) {
		if let Some(id) = self.id {
			remove_instance(self.mesh.as_ref().expect("Mesh is None!"), id);
			self.id = None;
		}
	}

	fn pre_change_mesh(&mut self, _owner: TRef<Spatial>) {
		if let Some(id) = self.id {
			remove_instance(self.mesh.as_ref().expect("Mesh is None!"), id);
			self.id = None;
		}
	}

	fn post_change_mesh(&mut self, owner: TRef<Spatial>) {
		if owner.is_inside_tree() {
			if let Some(mesh) = &self.mesh {
				let rid = { owner.get_world().unwrap() };
				let rid = unsafe { rid.assume_safe() };
				self.id = Some(add_instance(mesh.clone(), owner.claim(), rid));
			}
		}
	}
}

fn add_instance(mesh: Ref<Mesh>, node: Ref<Spatial>, world: TRef<gdnative::api::World>) -> usize {
	let vs = unsafe { VisualServer::godot_singleton() };
	let mut map = MULTI_MESHES.write().expect("Failed to access MULTI_MESHES");
	let id = map.0;
	let entry = map.1.entry(mesh.clone()).or_insert_with(|| {
		let mm_rid = vs.multimesh_create();
		let mesh_rid = unsafe { mesh.assume_safe().get_rid() };
		vs.multimesh_set_mesh(mm_rid, mesh_rid);
		vs.multimesh_allocate(
			mm_rid,
			MULTIMESH_ALLOC_STEP as i64,
			VisualServer::MULTIMESH_TRANSFORM_3D,
			VisualServer::MULTIMESH_COLOR_NONE,
			VisualServer::MULTIMESH_CUSTOM_DATA_NONE,
		);
		let inst_rid = vs.instance_create2(mm_rid, world.scenario());
		vs.instance_set_transform(
			inst_rid,
			Transform {
				basis: Basis::identity(),
				origin: Vector3::zero(),
			},
		);
		vs.instance_set_scenario(inst_rid, world.scenario());
		vs.instance_set_base(inst_rid, mm_rid);
		vs.instance_set_visible(inst_rid, true);
		(mm_rid, TypedArray::new(), Vec::new(), inst_rid)
	});
	entry.2.push((id, node));
	if entry.1.len() < entry.2.len() as i32 * 12 {
		let old_size = entry.1.len() / 12;
		let size = entry.1.len() / 12 + MULTIMESH_ALLOC_STEP as i32;
		entry.1.resize(size * 12);
		let mut w = entry.1.write();
		let trf = unsafe { node.assume_safe().global_transform() };
		for i in old_size..size {
			let i = i as usize * 12;
			for (k, &e) in transform_to_array(trf).iter().enumerate() {
				w[i + k] = e;
			}
			w[i] = 1.0;
			w[i + 5] = 1.0;
			w[i + 10] = 1.0;
		}
		drop(w);
		vs.multimesh_allocate(
			entry.0,
			size as i64,
			VisualServer::MULTIMESH_TRANSFORM_3D,
			VisualServer::MULTIMESH_COLOR_NONE,
			VisualServer::MULTIMESH_CUSTOM_DATA_NONE,
		);
		vs.multimesh_set_as_bulk_array(entry.0, entry.1.clone());
	}
	vs.multimesh_set_visible_instances(entry.0, entry.2.len() as i64);
	map.0 += 1;
	id
}

fn remove_instance(mesh: &Ref<Mesh>, id: usize) {
	let vs = unsafe { VisualServer::godot_singleton() };
	let mut map = MULTI_MESHES.write().expect("Failed to access MULTI_MESHES");
	let entry = map.1.get_mut(mesh).expect("Entry not found");
	let index = entry
		.2
		.binary_search_by(|(v, _)| v.cmp(&id))
		.expect("ID not found");
	entry.2.remove(index);
	vs.multimesh_set_visible_instances(entry.0, entry.2.len() as i64);
}

fn init(handle: InitHandle) {
	handle.add_tool_class::<BatchedMeshInstance>();
	handle.add_tool_class::<BatchedMeshManager>();
}

fn transform_to_array(transform: Transform) -> [f32; 12] {
	let tb = transform.basis;
	let to = transform.origin;
	let (bx, by, bz) = (tb.x(), tb.y(), tb.z());
	[
		bx.x, by.x, bz.x, to.x, bx.y, by.y, bz.y, to.y, bx.z, by.z, bz.z, to.z,
	]
}

godot_init!(init);
