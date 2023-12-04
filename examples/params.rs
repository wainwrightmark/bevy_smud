use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_asset_loader::prelude::*;
use bevy_smud::{prelude::*, param_usage::ShaderParamUsage, SmudShaders};
use rand::{prelude::IteratorRandom, random};

// this example shows how to use per-instance parameters in shapes
// in this simple example, a width and height is passed to a box shape,
// but it could be used for almost anything.

const PARAMS: usize = 2;

fn main() {
    App::new()
        .add_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::Loading).continue_to_state(GameState::Running),
        )
        .add_collection_to_loading_state::<_, AssetHandles>(GameState::Loading)
        .insert_resource(Msaa::Off)
        .add_plugins((DefaultPlugins, SmudPlugin::<PARAMS>, bevy_lospec::PalettePlugin))
        .add_systems(OnEnter(GameState::Running), setup)
        .run();
}

#[derive(Clone, Eq, PartialEq, Debug, Hash, States, Default)]
enum GameState {
    #[default]
    Loading,
    Running,
}

#[derive(Resource, AssetCollection)]
struct AssetHandles {
    #[asset(path = "vinik24.json")]
    palette: Handle<bevy_lospec::Palette>,
}

fn setup(
    mut commands: Commands,
    mut shaders: ResMut<Assets<Shader>>,
    assets: Res<AssetHandles>,
    palettes: Res<Assets<bevy_lospec::Palette>>,
) {
    let sdf_param_usage = ShaderParamUsage::all_params::<PARAMS>();


    let box_sdf = shaders.add_sdf_expr("smud::sd_box(p, vec2<f32>(param_0, param_1))", sdf_param_usage);
    let padding = 5.; // need some padding for the outline/falloff
    let spacing = 70.;
    let palette = palettes.get(&assets.palette).unwrap();

    let clear_color = palette.lightest();
    commands.insert_resource(ClearColor(clear_color));
    let mut rng = rand::thread_rng();

    for i in 0..100 {
        let size = Vec2::new(random::<f32>() * 20. + 1., random::<f32>() * 20. + 1.);
        let x = ((i % 10) as f32 - 4.5) * spacing;
        let y = ((i / 10) as f32 - 4.5) * spacing;

        let transform = Transform {
            scale: Vec3::splat(1.),
            translation: Vec3::new(x, y, 0.),
            rotation: Quat::from_rotation_z(random::<f32>() * PI),
        };

        let color = palette
            .iter()
            .filter(|c| *c != &clear_color)
            .choose(&mut rng)
            .copied()
            .unwrap_or(Color::PINK);

        commands.spawn(ShapeBundle::<PARAMS> {
            transform,
            shape: SmudShape {
                color,

                frame: Frame::Quad(size.x.max(size.y) + padding),
                params: [size.x.into(), size.y.into()],

                ..Default::default()
            },

            shaders: SmudShaders::<PARAMS> {
                sdf: box_sdf.clone(),
                // You can also specify a custom type of fill
                // The simple fill is just a simple anti-aliased opaque fill
                fill: SIMPLE_FILL_HANDLE,
                sdf_param_usage: sdf_param_usage,
                ..default()
            },

            ..Default::default()
        });
    }

    commands.spawn(Camera2dBundle::default());
}
