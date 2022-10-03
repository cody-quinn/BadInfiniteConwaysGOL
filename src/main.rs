#![warn(
    clippy::wildcard_imports,
    clippy::string_add,
    clippy::string_add_assign,
    clippy::manual_ok_or,
    unused_lifetimes
)]

mod input;
mod utils;

use bevy::log::{Level, LogSettings};
use bevy::prelude::{
    App, Assets, Camera2dBundle, Color, Commands, Component, ComputedVisibility, Entity, GlobalTransform, Handle,
    Input, KeyCode, Mesh, MouseButton, Query, Res, ResMut, SystemSet, Transform, Vec2, Visibility,
};
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::sprite::{ColorMaterial, Mesh2dHandle};
use bevy::time::FixedTimestep;
use bevy::utils::HashMap;
use bevy::window::WindowDescriptor;
use bevy::DefaultPlugins;
use bevy_inspector_egui::Inspectable;
#[cfg(debug_assertions)]
use bevy_inspector_egui::{RegisterInspectable, WorldInspectorPlugin};
use input::{CursorPanState, CursorPlugin, CursorPosition};
use utils::{from_chunk_pos, to_chunk_pos};

use crate::input::Camera;

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        // Setting a panic hook specific for WASM builds
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    }

    let mut app = App::new();

    // Constructing our app
    app.insert_resource(WindowDescriptor {
        title: "Game of Life".to_owned(),
        width: 1280.0,
        height: 720.0,
        ..Default::default()
    })
    .insert_resource(LogSettings {
        level: Level::INFO,
        ..Default::default()
    })
    .insert_resource(CursorPanState::default())
    .insert_resource(GlobalState::default())
    .insert_resource(CursorDrawState::default())
    .add_plugins(DefaultPlugins)
    .add_plugin(CursorPlugin)
    .add_startup_system(init_world)
    .add_system(input::handle_keyboard_pan_and_zoom)
    .add_system(input::handle_mouse_pan_and_zoom)
    .add_system(handle_click)
    .add_system(handle_play_pause)
    .add_system(tick_universe)
    .add_system_set(
        SystemSet::new()
            .with_run_criteria(FixedTimestep::steps_per_second(1.0))
            .with_system(tick_universe),
    );

    #[cfg(debug_assertions)]
    {
        // Adding the world inspector if debug mode is enabled
        app.add_plugin(WorldInspectorPlugin::new());
        app.register_inspectable::<Chunk>();
    }

    #[cfg(target_arch = "wasm32")]
    {
        // If the target is WASM adding the resizer plugin to scale with browser window
        app.add_plugin(bevy_web_resizer::Plugin);
    }

    // Running our app
    app.run();
}

fn init_world(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.spawn_bundle(Camera2dBundle::default()).insert(Camera);
    commands
        .spawn()
        .insert(Universe::new(materials.add(ColorMaterial::from(Color::GREEN))));
}

pub struct GlobalState {
    pub paused: bool,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self { paused: true }
    }
}

#[derive(Component)]
struct Universe {
    chunks: HashMap<(i32, i32), Chunk>,

    // Bevy stuff
    material: Handle<ColorMaterial>,
}

impl Universe {
    fn new(material: Handle<ColorMaterial>) -> Self {
        Self {
            chunks: HashMap::default(),
            material,
        }
    }

    fn spawn_chunk(&mut self, commands: &mut Commands, meshes: &mut ResMut<Assets<Mesh>>, chunk_pos: (i32, i32)) {
        let (world_x, world_y) = from_chunk_pos(chunk_pos);

        let mesh_handle = meshes.add(Mesh::new(PrimitiveTopology::TriangleList));
        let entity = commands
            .spawn()
            .insert(Mesh2dHandle(mesh_handle.clone()))
            .insert(self.material.clone())
            // Needed for thing to actually render
            .insert(Transform::from_xyz(world_x as f32, world_y as f32, 0.0))
            .insert(GlobalTransform::default())
            .insert(Visibility::default())
            .insert(ComputedVisibility::default())
            .id();
        let mut chunk = Chunk::new(chunk_pos, mesh_handle, entity);
        chunk.recalculate_mesh(meshes);

        self.chunks.insert(chunk_pos, chunk);
    }

    #[allow(dead_code)]
    fn despawn_chunk(&mut self, commands: &mut Commands, chunk_pos: (i32, i32)) {
        if let Some(chunk) = self.chunks.get(&chunk_pos) {
            commands.entity(chunk.entity).despawn();
            self.chunks.remove(&chunk_pos);
        }
    }

    fn get_chunk(
        &mut self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        chunk_pos: (i32, i32),
    ) -> &Chunk {
        if !self.chunks.contains_key(&chunk_pos) {
            self.spawn_chunk(commands, meshes, chunk_pos)
        }

        &self.chunks[&chunk_pos]
    }

    fn set_cell_state(
        &mut self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        world_pos: (f32, f32),
        state: bool,
    ) {
        let chunk_pos = to_chunk_pos(world_pos);
        let (x, y) = world_pos;

        if !self.chunks.contains_key(&chunk_pos) {
            self.spawn_chunk(commands, meshes, chunk_pos)
        }

        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            let local_x = ((50.0 + (x % 50.0)) % 50.0) as usize;
            let local_y = ((50.0 + (y % 50.0)) % 50.0) as usize;

            chunk.current_gen[local_x][local_y] = state;
        }
    }

    fn get_cell_state(&mut self, world_pos: (f32, f32)) -> bool {
        let chunk_pos = to_chunk_pos(world_pos);
        let (x, y) = world_pos;

        if let Some(chunk) = self.chunks.get_mut(&chunk_pos) {
            let local_x = ((50.0 + (x % 50.0)) % 50.0) as usize;
            let local_y = ((50.0 + (y % 50.0)) % 50.0) as usize;

            return chunk.current_gen[local_x][local_y];
        }

        false
    }

    #[allow(dead_code)]
    fn toggle_cell_state(&mut self, commands: &mut Commands, meshes: &mut ResMut<Assets<Mesh>>, world_pos: (f32, f32)) {
        let current_state = self.get_cell_state(world_pos);
        self.set_cell_state(commands, meshes, world_pos, !current_state);
    }

    fn tick(&mut self, commands: &mut Commands, meshes: &mut ResMut<Assets<Mesh>>) {
        // Prepare every chunk for being ticked
        for (_, chunk) in &mut self.chunks {
            chunk.prepare_tick()
        }

        // Spawning needed chunks
        let mut needed_chunks = Vec::<(i32, i32)>::new();
        for ((x, y), chunk) in &mut self.chunks {
            if chunk.last_gen_alive > 0 {
                needed_chunks.push((x - 1, *y));
                needed_chunks.push((x - 1, y + 1));
                needed_chunks.push((*x, y + 1));
                needed_chunks.push((x + 1, y + 1));
                needed_chunks.push((x + 1, *y));
                needed_chunks.push((x + 1, y - 1));
                needed_chunks.push((*x, y - 1));
                needed_chunks.push((x - 1, y - 1));
            }
        }

        for pos in needed_chunks {
            self.get_chunk(commands, meshes, pos);
        }

        // Get a frozen version of all the chunk data
        let chunk_data = self
            .chunks
            .iter()
            .map(|(pos, chunk)| (*pos, chunk.last_gen))
            .collect::<HashMap<_, _>>();

        for ((x, y), chunk) in &mut self.chunks {
            chunk.tick([
                *chunk_data.get(&(x - 1, *y)).unwrap_or(&[[false; 50]; 50]),
                *chunk_data.get(&(x - 1, y + 1)).unwrap_or(&[[false; 50]; 50]),
                *chunk_data.get(&(*x, y + 1)).unwrap_or(&[[false; 50]; 50]),
                *chunk_data.get(&(x + 1, y + 1)).unwrap_or(&[[false; 50]; 50]),
                *chunk_data.get(&(x + 1, *y)).unwrap_or(&[[false; 50]; 50]),
                *chunk_data.get(&(x + 1, y - 1)).unwrap_or(&[[false; 50]; 50]),
                *chunk_data.get(&(*x, y - 1)).unwrap_or(&[[false; 50]; 50]),
                *chunk_data.get(&(x - 1, y - 1)).unwrap_or(&[[false; 50]; 50]),
            ]);
        }

        for (_, chunk) in &mut self.chunks {
            if chunk.changed() {
                chunk.recalculate_mesh(meshes);
            }
        }
    }
}

fn tick_universe(
    mut universe: Query<&mut Universe>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    state: Res<GlobalState>,
) {
    if let Ok(mut universe) = universe.get_single_mut() {
        if !state.paused {
            universe.tick(&mut commands, &mut meshes);
        } else {
            for (_, chunk) in &mut universe.chunks {
                chunk.recalculate_mesh(&mut meshes);
            }
        }
    }
}

#[derive(Default)]
pub struct CursorDrawState {
    cell_state: bool,
}

fn handle_click(
    mouse_btn_input: Res<Input<MouseButton>>,
    cursor_pos: Res<CursorPosition>,
    mut universe: Query<&mut Universe>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut draw_state: ResMut<CursorDrawState>,
    state: Res<GlobalState>,
) {
    if !state.paused {
        return;
    }

    if let Ok(mut universe) = universe.get_single_mut() {
        if let Some(Vec2 { x, y }) = cursor_pos.0 {
            if mouse_btn_input.just_pressed(MouseButton::Left) {
                draw_state.cell_state = !universe.get_cell_state((x, y));
            }

            if mouse_btn_input.pressed(MouseButton::Left) {
                universe.set_cell_state(&mut commands, &mut meshes, (x, y), draw_state.cell_state);
            }
        }
    }
}

fn handle_play_pause(
    keyboard_input: Res<Input<KeyCode>>,
    mut state: ResMut<GlobalState>,
    mut universe: Query<&mut Universe>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        state.paused = !state.paused
    }

    if keyboard_input.just_pressed(KeyCode::Right) {
        if let Ok(mut universe) = universe.get_single_mut() {
            universe.tick(&mut commands, &mut meshes);
        }
    }
}

#[derive(Component, Inspectable)]
struct Chunk {
    pos: (i32, i32),

    // Previous generation stuff
    last_gen: [[bool; 50]; 50],
    last_gen_alive: i32,

    // Current generation stuff
    current_gen: [[bool; 50]; 50],
    current_gen_alive: i32,

    // Bevy things
    mesh: Handle<Mesh>,
    entity: Entity,
}

/// When ticking a chunk run prepare_tick for all, then tick for all and then
/// recalculate mesh.
impl Chunk {
    fn new(pos: (i32, i32), mesh: Handle<Mesh>, entity: Entity) -> Self {
        Self {
            pos,
            last_gen: [[false; 50]; 50],
            last_gen_alive: 0,
            current_gen: [[false; 50]; 50],
            current_gen_alive: 0,
            mesh,
            entity,
        }
    }

    fn prepare_tick(&mut self) {
        // In preperation for next tick copy data from current generation to last
        // generation.
        self.last_gen = self.current_gen;
        self.last_gen_alive = self.current_gen_alive;

        self.current_gen = [[false; 50]; 50];
        self.current_gen_alive = 0;
    }

    // 0: chunk_data.get(&(x - 1, y    )).unwrap_or(&[[false; 50]; 50]),
    // 1: chunk_data.get(&(x - 1, y + 1)).unwrap_or(&[[false; 50]; 50]),
    // 2: chunk_data.get(&(x    , y + 1)).unwrap_or(&[[false; 50]; 50]),
    // 3: chunk_data.get(&(x + 1, y + 1)).unwrap_or(&[[false; 50]; 50]),
    // 4: chunk_data.get(&(x + 1, y    )).unwrap_or(&[[false; 50]; 50]),
    // 5: chunk_data.get(&(x + 1, y - 1)).unwrap_or(&[[false; 50]; 50]),
    // 6: chunk_data.get(&(x    , y - 1)).unwrap_or(&[[false; 50]; 50]),
    // 7: chunk_data.get(&(x - 1, y - 1)).unwrap_or(&[[false; 50]; 50]),

    #[rustfmt::skip]
    fn get_neighbor_status(&self, chunk_neighbors_state: &[[[bool; 50]; 50]; 8], pos: (i32, i32)) -> bool {
        match pos {
            (-1, 50) => chunk_neighbors_state[1][49][0],
            (50, 50) => chunk_neighbors_state[3][0][0],
            (50, -1) => chunk_neighbors_state[5][0][49],
            (-1, -1) => chunk_neighbors_state[7][49][49],
            (-1,  y) => chunk_neighbors_state[0][49][y as usize],
            ( x, 50) => chunk_neighbors_state[2][x as usize][0],
            (50,  y) => chunk_neighbors_state[4][0][y as usize],
            ( x, -1) => chunk_neighbors_state[6][x as usize][49],
            ( x,  y) => self.last_gen[x as usize][y as usize],
        }
    }

    #[rustfmt::skip]
    fn get_alive_neighbors(&self, chunk_neighbors_state: &[[[bool; 50]; 50]; 8], (x, y): (i32, i32)) -> i32 {
        let mut total_alive = 0;
        total_alive += self.get_neighbor_status(chunk_neighbors_state, (x - 1, y    )) as i32;
        total_alive += self.get_neighbor_status(chunk_neighbors_state, (x - 1, y + 1)) as i32;
        total_alive += self.get_neighbor_status(chunk_neighbors_state, (x    , y + 1)) as i32;
        total_alive += self.get_neighbor_status(chunk_neighbors_state, (x + 1, y + 1)) as i32;
        total_alive += self.get_neighbor_status(chunk_neighbors_state, (x + 1, y    )) as i32;
        total_alive += self.get_neighbor_status(chunk_neighbors_state, (x + 1, y - 1)) as i32;
        total_alive += self.get_neighbor_status(chunk_neighbors_state, (x    , y - 1)) as i32;
        total_alive += self.get_neighbor_status(chunk_neighbors_state, (x - 1, y - 1)) as i32;
        total_alive
    }

    fn tick(
        &mut self,
        chunk_neighbors_state: [[[bool; 50]; 50]; 8],
        // Clockwise starting west - universe: &mut Universe,
    ) {
        for x in 0..50 {
            for y in 0..50 {
                let alive = self.last_gen[x as usize][y as usize];
                let alive_neighbors = self.get_alive_neighbors(&chunk_neighbors_state, (x, y));

                let now_alive = match alive_neighbors {
                    2 | 3 if alive => true,
                    3 if !alive => true,
                    _ => false,
                };

                self.current_gen[x as usize][y as usize] = now_alive;

                if now_alive {
                    self.current_gen_alive += 1;
                }
            }
        }
    }

    fn recalculate_mesh(&mut self, meshes: &mut ResMut<Assets<Mesh>>) {
        let mut verticies = Vec::<([f32; 3], [f32; 3], [f32; 2])>::with_capacity(50 * 50 * 4);
        let mut indicies = Vec::<u32>::with_capacity(50 * 50 * 6);

        let mut index = 0;
        for x in 0..50 {
            for y in 0..50 {
                let alive = self.current_gen[x][y];

                if alive {
                    // Adding the veriticies
                    let y0 = y as f32;
                    let y1 = y as f32 + 1.0;
                    let x0 = x as f32;
                    let x1 = x as f32 + 1.0;

                    verticies.push(([x0, y0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0]));
                    verticies.push(([x0, y1, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0]));
                    verticies.push(([x1, y1, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0]));
                    verticies.push(([x1, y0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]));

                    // Adding the indicies
                    indicies.push(4 * index as u32);
                    indicies.push(4 * index as u32 + 2);
                    indicies.push(4 * index as u32 + 1);
                    indicies.push(4 * index as u32);
                    indicies.push(4 * index as u32 + 3);
                    indicies.push(4 * index as u32 + 2);

                    // Increasing the index
                    index += 1;
                }
            }
        }

        let verticies_positions = verticies.iter().map(|(p, _, _)| *p).collect::<Vec<_>>();
        let verticies_normals = verticies.iter().map(|(_, n, _)| *n).collect::<Vec<_>>();
        let verticies_uv = verticies.iter().map(|(_, _, u)| *u).collect::<Vec<_>>();

        if let Some(mesh) = meshes.get_mut(&self.mesh) {
            mesh.set_indices(Some(Indices::U32(indicies)));
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, verticies_positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, verticies_normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, verticies_uv);
        }
    }

    fn changed(&self) -> bool {
        self.current_gen != self.last_gen
    }
}
