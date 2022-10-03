use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::{
    App, Camera2d, Component, Deref, DerefMut, EventReader, Input, KeyCode, MouseButton, Plugin, Query, Res, ResMut,
    Transform, Vec2, With,
};
use bevy::time::Time;
use bevy::window::{CursorMoved, Windows};
use num::clamp;

#[derive(Component)]
pub struct Camera;

/// Simple script that handles panning with the keyboard.
pub fn handle_keyboard_pan_and_zoom(
    mut cameras: Query<&mut Transform, With<Camera>>,
    timer: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    if keyboard_input.pressed(KeyCode::W) {
        for mut transform in &mut cameras {
            transform.translation.y += 1000.0 * timer.delta_seconds();
        }
    }

    if keyboard_input.pressed(KeyCode::A) {
        for mut transform in &mut cameras {
            transform.translation.x -= 1000.0 * timer.delta_seconds();
        }
    }

    if keyboard_input.pressed(KeyCode::S) {
        for mut transform in &mut cameras {
            transform.translation.y -= 1000.0 * timer.delta_seconds();
        }
    }

    if keyboard_input.pressed(KeyCode::D) {
        for mut transform in &mut cameras {
            transform.translation.x += 1000.0 * timer.delta_seconds();
        }
    }

    if keyboard_input.pressed(KeyCode::Q) {
        for mut transform in &mut cameras {
            let new_scale = transform.scale.x + (3.5 * timer.delta_seconds());
            transform.scale.x = clamp(new_scale, 0.05, 10.0);
            transform.scale.y = clamp(new_scale, 0.05, 10.0);
        }
    }

    if keyboard_input.pressed(KeyCode::E) {
        for mut transform in &mut cameras {
            let new_scale = transform.scale.x - (3.5 * timer.delta_seconds());
            transform.scale.x = clamp(new_scale, 0.05, 10.0);
            transform.scale.y = clamp(new_scale, 0.05, 10.0);
        }
    }
}

#[derive(Default)]
pub struct CursorPanState {
    last_pos: Option<Vec2>,
}

/// Simple system that handles mouse panning and zooming. You can zoom with the
/// scrolling wheel and pan by holding down left click on the mouse.
///
/// An improvement that could be made is zooming on the user's mouse cursor.
pub fn handle_mouse_pan_and_zoom(
    mut cameras: Query<&mut Transform, With<Camera>>,
    mouse_btn_input: Res<Input<MouseButton>>,
    mut cursor_move_events: EventReader<CursorMoved>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut pan_state: ResMut<CursorPanState>,
    timer: Res<Time>,
) {
    if mouse_btn_input.pressed(MouseButton::Right) && !cursor_move_events.is_empty() {
        let curr_pos = cursor_move_events.iter().last().map(|it| it.position);

        if let Some(curr_pos) = curr_pos {
            if let Some(last_pos) = &pan_state.last_pos {
                let delta = curr_pos - *last_pos;

                if delta != Vec2::ZERO {
                    for mut transform in &mut cameras {
                        transform.translation.x -= delta.x * transform.scale.x;
                        transform.translation.y -= delta.y * transform.scale.y;
                    }
                }
            }
        }

        pan_state.last_pos = curr_pos;
    } else {
        pan_state.last_pos = None;
    }

    if !mouse_wheel_events.is_empty() {
        let scroll_sum = mouse_wheel_events
            .iter()
            .map(|it| {
                if it.unit == MouseScrollUnit::Line {
                    it.y * 50.0
                } else {
                    it.y
                }
            })
            .sum::<f32>();

        for mut transform in &mut cameras {
            let new_scale = transform.scale.x - (scroll_sum * 0.25 * timer.delta_seconds());
            transform.scale.x = clamp(new_scale, 0.05, 10.0);
            transform.scale.y = clamp(new_scale, 0.05, 10.0);
        }
    }
}

/// Shamelessly stolen from https://discord.com/channels/691052431525675048/996942216444518481/996944143139995748
pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorPosition>().add_system(get_cursor_position);
    }
}

#[derive(Debug, Default, Deref, DerefMut)]
pub struct CursorPosition(pub Option<Vec2>);

fn get_cursor_position(
    cameras: Query<&Transform, With<Camera2d>>,
    windows: Res<Windows>,
    mut position: ResMut<CursorPosition>,
) {
    if let Ok(transform) = cameras.get_single() {
        let window = windows.get_primary().unwrap();
        **position = window.cursor_position().map(|cursor_position| {
            (transform.compute_matrix()
                * (cursor_position - Vec2::new(window.width(), window.height()) / 2.)
                    .extend(0.)
                    .extend(1.))
            .truncate()
            .truncate()
        })
    }
}
