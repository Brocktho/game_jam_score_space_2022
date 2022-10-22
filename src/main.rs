use bevy::prelude::{Camera2dBundle, *};
use bevy::render::camera::ScalingMode;
use bevy::sprite;
use heron::{prelude::*, PhysicsSteps};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(PhysicsPlugin::default())
        .insert_resource(Gravity::from(Vec3::new(0.0, -9.81, 0.0)))
        .insert_resource(PhysicsTime::new(1.5))
        .insert_resource(PhysicsSteps::from_steps_per_seconds(30.))
        .add_startup_system(create_character)
        .add_startup_system(setup_camera)
        .add_system(move_player)
        .add_startup_system(create_floor)
        .run();
}

#[derive(PhysicsLayer)]
enum Layers {
    World,
    Player,
    Enemies,
}

#[derive(Component)]
pub struct Player;

pub fn setup_camera(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle {
        projection: OrthographicProjection {
            scale: 250.0,
            scaling_mode: ScalingMode::FixedVertical(1.),
            ..default()
        },
        ..default()
    });
}

pub fn create_floor(mut commands: Commands, asset_server: Res<AssetServer>) {
    let floor_sprite: Handle<Image> = asset_server.load("images/Floor.png");
    let floor_size = Vec2::new(400.0, 10.0);
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(floor_size),
                ..default()
            },
            texture: floor_sprite.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..default()
        })
        .insert(RigidBody::Static)
        .insert(CollisionShape::Cuboid {
            half_extends: floor_size.extend(0.0) / 2.,
            border_radius: None,
        })
        .insert(
            CollisionLayers::none()
                .with_group(Layers::World)
                .with_mask(Layers::Player),
        );
}

pub fn create_character(mut commands: Commands, asset_server: Res<AssetServer>) {
    let character_sprite: Handle<Image> = asset_server.load("images/Character.png");
    let sprite_size = Vec2::new(28.0, 28.0);
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(sprite_size),
                ..Default::default()
            },
            texture: character_sprite.clone(),
            transform: Transform::from_translation(Vec3::splat(1.0)),
            ..default()
        })
        .insert(RigidBody::Dynamic)
        .insert(CollisionShape::Cuboid {
            half_extends: sprite_size.extend(0.0) / 2.0,
            border_radius: None,
        })
        .insert(Player)
        .insert(Velocity { ..default() })
        .insert(Acceleration::default())
        .insert(
            CollisionLayers::none()
                .with_group(Layers::Player)
                .with_mask(Layers::World),
        );
}

pub fn move_player(keys: Res<Input<KeyCode>>, mut player_query: Query<&mut Transform, &Player>) {
    let player_check = player_query.iter_mut().next();

    match player_check {
        Some(mut player) => {
            if keys.pressed(KeyCode::D) {
                player.translation.x += 1.0;
            } else if keys.pressed(KeyCode::A) {
                player.translation.x -= 1.0;
            }
        }
        _default => {} // do nothing
    }
}
