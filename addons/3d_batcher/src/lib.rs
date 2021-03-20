use gdnative::api::{Mesh, Node, VisualServer};
use gdnative::prelude::*;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::RwLock;

const MULTIMESH_ALLOC_STEP: usize = 256;

lazy_static! {
	static ref MULTI_MESHES: RwLock<State> = RwLock::new(State {
		id_counter: 0,
		no_color_map: HashMap::new(),
		color_map: HashMap::new()
	});
}

struct NodeEntry {
	id: usize,
	node: Instance<BatchedMeshInstance, Shared>,
}

struct MultiMeshEntry {
	instance_rid: Rid,
	multimesh_rid: Rid,
	visualserver_data: TypedArray<f32>,
	nodes: Vec<NodeEntry>,
}

struct State {
	id_counter: usize,
	no_color_map: HashMap<Ref<Mesh>, MultiMeshEntry>,
	color_map: HashMap<Ref<Mesh>, MultiMeshEntry>,
}

#[derive(NativeClass)]
#[inherit(Node)]
struct BatchedMeshManager {}

#[derive(NativeClass)]
#[inherit(Spatial)]
#[register_with(Self::register)]
struct BatchedMeshInstance {
	#[property(
		before_set = "Self::pre_change_mesh",
		after_set = "Self::post_change_mesh"
	)]
	mesh: Option<Ref<Mesh>>,
	id: Option<usize>,
	#[property(after_set = "Self::toggled_use_color")]
	use_color: bool,
	color: [u8; 4],
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
		for entry in map.no_color_map.values_mut() {
			let mut w = entry.visualserver_data.write();
			for (i, e) in entry.nodes.iter().enumerate() {
				unsafe {
					e.node
						.assume_safe()
						.map(|_, o| {
							let trf = o.global_transform();
							for (k, &e) in transform_to_array(trf).iter().enumerate() {
								w[i * 12 + k] = e;
							}
						})
						.expect("Failed to borrow node");
				}
			}
			drop(w);
			vs.multimesh_set_as_bulk_array(entry.multimesh_rid, entry.visualserver_data.clone());
		}
		for entry in map.color_map.values_mut() {
			let mut w = entry.visualserver_data.write();
			for (i, e) in entry.nodes.iter().enumerate() {
				unsafe {
					e.node
						.assume_safe()
						.map(|s, o| {
							let trf = o.global_transform();
							for (k, &e) in transform_to_array(trf).iter().enumerate() {
								w[i * 13 + k] = e;
							}
							// TODO is it always little endian?
							w[i * 13 + 12] = f32::from_le_bytes(s.color);
						})
						.expect("Failed to borrow node");
				}
			}
			drop(w);
			vs.multimesh_set_as_bulk_array(entry.multimesh_rid, entry.visualserver_data.clone());
		}
	}
}

#[methods]
impl BatchedMeshInstance {
	fn register(builder: &ClassBuilder<Self>) {
		builder
			.add_property("color")
			.with_getter(Self::gd_get_color)
			.with_setter(Self::gd_set_color)
			.done();
	}

	fn new(_owner: TRef<Spatial>) -> Self {
		Self {
			mesh: None,
			id: None,
			use_color: false,
			color: [255; 4],
		}
	}

	#[export]
	fn _enter_tree(&mut self, owner: TRef<Spatial>) {
		debug_assert_eq!(self.id, None);
		if let Some(mesh) = &self.mesh {
			let rid = { owner.get_world().unwrap() };
			let rid = unsafe { rid.assume_safe() };
			self.id = Some(add_instance(
				mesh.clone(),
				owner.cast_instance().unwrap().claim(),
				rid,
				self.use_color,
			));
		}
	}

	#[export]
	fn _exit_tree(&mut self, _owner: TRef<Spatial>) {
		if let Some(id) = self.id {
			remove_instance(
				self.mesh.as_ref().expect("Mesh is None!"),
				id,
				self.use_color,
			);
			self.id = None;
		}
	}

	fn pre_change_mesh(&mut self, _owner: TRef<Spatial>) {
		if let Some(id) = self.id {
			remove_instance(
				self.mesh.as_ref().expect("Mesh is None!"),
				id,
				self.use_color,
			);
			self.id = None;
		}
	}

	fn post_change_mesh(&mut self, owner: TRef<Spatial>) {
		if owner.is_inside_tree() {
			if let Some(mesh) = &self.mesh {
				let rid = { owner.get_world().unwrap() };
				let rid = unsafe { rid.assume_safe() };
				self.id = Some(add_instance(
					mesh.clone(),
					owner.cast_instance().unwrap().claim(),
					rid,
					self.use_color,
				));
			}
		}
	}

	fn toggled_use_color(&mut self, owner: TRef<Spatial>) {
		if let Some(id) = self.id {
			let mesh = self.mesh.as_ref().expect("Mesh is None!");
			remove_instance(mesh, id, !self.use_color);
			let rid = { owner.get_world().unwrap() };
			let rid = unsafe { rid.assume_safe() };
			self.id = Some(add_instance(
				mesh.clone(),
				owner.cast_instance().unwrap().claim(),
				rid,
				self.use_color,
			));
		}
	}

	fn gd_get_color(&self, _owner: TRef<Spatial>) -> Color {
		let [r, g, b, a] = self.color;
		let (r, g, b, a) = (r as f32, g as f32, b as f32, a as f32);
		let (r, g, b, a) = (r / 255.0, g / 255.0, b / 255.0, a / 255.0);
		Color::rgba(r, g, b, a)
	}

	fn gd_set_color(&mut self, owner: TRef<Spatial>, color: Color) {
		let Color { r, g, b, a } = color;
		let (r, g, b, a) = (r * 255.0, g * 255.0, b * 255.0, a * 255.0);
		let (r, g, b, a) = (r as u8, g as u8, b as u8, a as u8);
		self.color = [r, g, b, a];
		if self.use_color {
			if let Some(id) = self.id {
				let mesh = self.mesh.as_ref().expect("Mesh is None!");
				remove_instance(mesh, id, self.use_color);
				let rid = { owner.get_world().unwrap() };
				let rid = unsafe { rid.assume_safe() };
				self.id = Some(add_instance(
					mesh.clone(),
					owner.cast_instance().unwrap().claim(),
					rid,
					self.use_color,
				));
			}
		}
	}
}

fn add_instance(
	mesh: Ref<Mesh>,
	node: Instance<BatchedMeshInstance, Shared>,
	world: TRef<gdnative::api::World>,
	use_color: bool,
) -> usize {
	let vs = unsafe { VisualServer::godot_singleton() };
	let mut map = MULTI_MESHES.write().expect("Failed to access MULTI_MESHES");
	let id = map.id_counter;
	map.id_counter += 1;

	let (map, instance_data_size, color_setting) = if use_color {
		(&mut map.color_map, 13, VisualServer::MULTIMESH_COLOR_8BIT)
	} else {
		(
			&mut map.no_color_map,
			12,
			VisualServer::MULTIMESH_COLOR_NONE,
		)
	};

	let entry = map.entry(mesh.clone()).or_insert_with(|| {
		let mm_rid = vs.multimesh_create();
		let mesh_rid = unsafe { mesh.assume_safe().get_rid() };
		vs.multimesh_set_mesh(mm_rid, mesh_rid);
		vs.multimesh_allocate(
			mm_rid,
			MULTIMESH_ALLOC_STEP as i64,
			VisualServer::MULTIMESH_TRANSFORM_3D,
			color_setting,
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
		MultiMeshEntry {
			instance_rid: inst_rid,
			multimesh_rid: mm_rid,
			visualserver_data: TypedArray::new(),
			nodes: Vec::new(),
		}
	});

	entry.nodes.push(NodeEntry { id, node });

	let size = entry.visualserver_data.len() as usize / instance_data_size;
	if entry.nodes.len() > size {
		let new_size = size + MULTIMESH_ALLOC_STEP;
		entry
			.visualserver_data
			.resize((new_size * instance_data_size) as i32);
		let mut w = entry.visualserver_data.write();
		for i in size..new_size {
			let i = i as usize * instance_data_size;
			for k in 0..instance_data_size {
				w[i + k] = 0.0;
			}
			w[i] = 1.0;
			w[i + 5] = 1.0;
			w[i + 10] = 1.0;
		}
		drop(w);
		vs.multimesh_allocate(
			entry.multimesh_rid,
			new_size as i64,
			VisualServer::MULTIMESH_TRANSFORM_3D,
			color_setting,
			VisualServer::MULTIMESH_CUSTOM_DATA_NONE,
		);
		vs.multimesh_set_as_bulk_array(entry.multimesh_rid, entry.visualserver_data.clone());
	}
	vs.multimesh_set_visible_instances(entry.multimesh_rid, entry.nodes.len() as i64);

	id
}

fn remove_instance(mesh: &Ref<Mesh>, id: usize, uses_color: bool) {
	let vs = unsafe { VisualServer::godot_singleton() };
	let mut map = MULTI_MESHES.write().expect("Failed to access MULTI_MESHES");
	let map = if uses_color {
		&mut map.color_map
	} else {
		&mut map.no_color_map
	};
	let entry = map.get_mut(mesh).expect("Entry not found");
	let index = entry
		.nodes
		.binary_search_by(|e| e.id.cmp(&id))
		.expect("ID not found");
	entry.nodes.remove(index);
	vs.multimesh_set_visible_instances(entry.multimesh_rid, entry.nodes.len() as i64);
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
