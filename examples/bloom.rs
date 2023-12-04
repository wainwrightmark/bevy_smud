//! This example shows that bevy_smud works with bloom enabled
//!
//! Note that you could probably achieve cheaper and higher quality bloom-like
//! effects by creating a custom fill.

use bevy::{core_pipeline::bloom::BloomSettings, prelude::*};
// The prelude contains the basic things needed to create shapes
use bevy_smud::{prelude::*, param_usage::ShaderParamUsage, SmudShaders};

fn main() {
    App::new()
        // bevy_smud comes with anti-aliasing built into the standards fills
        // which is more efficient than MSAA, and also works on Linux, wayland
        .insert_resource(Msaa::Off)
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((DefaultPlugins, SmudPlugin::<0,0>))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut shaders: ResMut<Assets<Shader>>) {
    // add_sdf_expr expects a wgsl expression
    // p is the position of a fragment within the sdf shape, with 0, 0 at the center.
    // Here we are using the built-in sd_circle function, which accepts the
    // radius as a parameter.
    let circle = shaders.add_sdf_expr("smud::sd_circle(p, 70.)", ShaderParamUsage::NO_PARAMS);

    commands.spawn(ShapeBundle {
        shape: SmudShape::<0,0> {
            color: Color::TOMATO,
            // The frame needs to be bigger than the shape we're drawing
            // Since the circle has radius 70, we make the half-size of the quad 80.
            frame: Frame::Quad(80.),

            ..Default::default()
        },
        shaders: SmudShaders::<0,0>{
            sdf: circle,
            fill: SIMPLE_FILL_HANDLE,
            ..default()
        },
        ..default()
    });

    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        BloomSettings {
            intensity: 0.7,
            ..default()
        },
    ));
}
