use std::time::Duration;

use bevy::prelude::{Camera2dBundle, *};
use bevy::render::camera::{RenderTarget, ScalingMode};
use bevy_inspector_egui::{Inspectable, InspectorPlugin};
use heron::{prelude::*, PhysicsSteps};
use libm::{atan2f, cosf, sinf};
use rand::Rng;

#[derive(Component, Inspectable)]
pub struct GravityData {
    #[inspectable(min = 0.1, max = 1000.0)]
    phys_time: f32,
    #[inspectable(min = 1., max = 10000.0)]
    phys_step: f32,
    #[inspectable(min = Vec3::splat(-1000.0), max = Vec3::splat(1000.0))]
    gravity: Vec3,
    #[inspectable(min = Vec3::splat(-1000.0), max = Vec3::splat(1000.0))]
    gun_force: Vec3,
    #[inspectable(min = Vec3::new(0.,-92.0,0.0), max = Vec3::splat(100.0) )]
    player_pos: Vec3,
    #[inspectable(min = 1.0, max = 100000.0)]
    jump_height: f32,
    #[inspectable(min = 0.1, max = 2.0)]
    weapon_time: f32,
}

impl Default for GravityData {
    fn default() -> Self {
        GravityData {
            phys_time: 1.,
            phys_step: 30.,
            gravity: Vec3::new(0., -70.1, 0.),
            gun_force: Vec3::new(0.0, 0.0, 0.0),
            player_pos: Vec3::new(0., -92.0, 0.0),
            jump_height: 100.0,
            weapon_time: 1.0,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum Directions {
    Left,
    Right,
}

pub struct DashTimer {
    timer: Timer,
    direction: Directions,
}

impl Default for DashTimer {
    fn default() -> Self {
        DashTimer {
            timer: Timer::from_seconds(0.5, false),
            direction: Directions::Left,
        }
    }
}

#[derive(Copy, Clone)]
pub enum Weapons {
    Base,
    Rocket,
    Sniper,
    Shotgun,
    Rock,
    Airplane,
}

#[derive(Component, Clone, Copy)]
pub struct Weapon {
    asset: Weapons,
}

pub struct Score {
    score: i64,
}

impl Default for Score {
    fn default() -> Self {
        Score { score: 0 }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(PhysicsPlugin::default())
        .add_plugin(InspectorPlugin::<GravityData>::new())
        .insert_resource(GravityData::default())
        .insert_resource(Gravity::from(Vec3::new(0.0, -70.1, 0.0)))
        .insert_resource(PhysicsTime::new(1.))
        .insert_resource(PhysicsSteps::from_steps_per_seconds(30.))
        .insert_resource(WeaponSpawns {
            timer: Timer::from_seconds(1.0, true),
        })
        .insert_resource(Score::default())
        .add_system(fire_weapon)
        .insert_resource(DashTimer {
            timer: Timer::from_seconds(0.0001, false),
            direction: Directions::Left,
        })
        .add_startup_system(create_character)
        .add_system(update_physics)
        .add_startup_system(setup_camera)
        .add_system(tick_timers)
        .add_system(move_player)
        .add_startup_system(create_floor)
        .add_system(grab_weapon)
        .add_system(point_held_item)
        .run();
}

pub fn update_physics(
    phys_data: Res<GravityData>,
    mut gravity: ResMut<Gravity>,
    mut phys_time: ResMut<PhysicsTime>,
    mut phys_step: ResMut<PhysicsSteps>,
    mut player: Query<(&mut Transform, &mut Velocity, &mut Player)>,
    mut gun_time: ResMut<WeaponSpawns>,
) {
    if !phys_data.is_changed() {
        return;
    }
    *gravity = Gravity::from(phys_data.gravity.clone());
    phys_time.set_scale(phys_data.phys_time);
    *phys_step = PhysicsSteps::from_steps_per_seconds(phys_data.phys_step);
    let check_play = player.iter_mut().next();
    match check_play {
        Some((mut trans, mut vel, mut play)) => {
            trans.translation = phys_data.player_pos;
            play.jump_height = phys_data.jump_height;
        }
        _default => {} // do nothing
    }
    *gun_time = WeaponSpawns {
        timer: Timer::from_seconds(phys_data.weapon_time, true),
    }
}

pub fn weapon_enum_to_string(weapon: Weapons) -> String {
    let mut output = String::with_capacity(30);
    match weapon {
        Weapons::Base => output += "images/BaseGun.png",
        Weapons::Sniper => output += "images/Sniper.png",
        Weapons::Rock => output += "images/Rock.png",
        Weapons::Airplane => output += "images/Airplane.png",
        Weapons::Shotgun => output += "images/Shotgun.png",
        Weapons::Rocket => output += "images/Rocket.png",
        //_defualt => output += "",
    }
    output
}

pub fn spawn_weapon(mut commands: Commands, texture: Handle<Image>) {
    let weapon_size = Vec2::new(14., 4.);
    let mut random = rand::thread_rng();
    let random_x = random.gen_range(-120.0..120.0) as f32;
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform::from_translation(Vec3::new(random_x, 300.0, 0.0)),
            sprite: Sprite {
                custom_size: Some(weapon_size),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn_bundle(SpriteBundle {
                    texture: texture.clone(),
                    sprite: Sprite {
                        custom_size: Some(weapon_size),
                        ..default()
                    },
                    ..default()
                })
                .insert(RigidBody::Sensor)
                .insert(CollisionShape::Sphere { radius: 15.0 })
                .insert(
                    CollisionLayers::none()
                        .with_group(Layers::Weapons)
                        .with_mask(Layers::Player),
                )
                .insert(Weapon {
                    asset: Weapons::Base,
                })
                .insert(Collisions::default());
        })
        .insert(RigidBody::Dynamic)
        .insert(CollisionShape::Cuboid {
            half_extends: weapon_size.extend(0.) / 2.0,
            border_radius: None,
        })
        .insert(CollisionLayers::none().with_group(Layers::Weapons));
}

pub fn tick_timers(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut weapon_time: ResMut<WeaponSpawns>,
    mut dash_time: ResMut<DashTimer>,
    mut any_dashing: Query<(Entity, &mut Dashing), With<Dashing>>,
    mut any_bullets: Query<(Entity, &mut Bullet)>,
    time: Res<Time>,
) {
    any_dashing.iter_mut().for_each(|(dasher, mut dashing)| {
        dashing.timer.tick(time.delta());
        if dashing.timer.finished() {
            commands.entity(dasher).remove::<Dashing>();
        }
    });
    any_bullets.iter_mut().for_each(|(entity, mut bullet)| {
        bullet.timer.tick(time.delta());
        if bullet.timer.finished() {
            commands.entity(entity).despawn_recursive();
        }
    });
    dash_time.timer.tick(time.delta());
    weapon_time.timer.tick(time.delta());
    if weapon_time.timer.finished() {
        let weapon_sprite: Handle<Image> = asset_server.load("images/BaseGun.png");
        spawn_weapon(commands, weapon_sprite.clone());
    }
}

pub fn grab_weapon(
    mut commands: Commands,
    weapons: Query<(Entity, &Collisions, &Weapon)>,
    player_query: Query<&Player>,
    asset_server: Res<AssetServer>,
    query_held_item: Query<(Entity, &Weapon), With<HeldItem>>,
) {
    let player_check = player_query.iter().next();
    match player_check {
        Some(player) => {
            weapons.iter().for_each(|(entity, collisions, &weapon)| {
                collisions.entities().for_each(|collision| {
                    query_held_item.iter().for_each(|(held_item, &weapon)| {
                        let asset_str = weapon_enum_to_string(weapon.asset);
                        let thrown_sprite: Handle<Image> = asset_server.load(&asset_str);
                        commands.entity(held_item).despawn_recursive();
                        commands
                            .spawn_bundle(SpriteBundle {
                                transform: Transform::from_translation(player.location),
                                texture: thrown_sprite,
                                ..default()
                            })
                            .insert(RigidBody::Dynamic)
                            .insert(CollisionShape::Cuboid {
                                half_extends: Vec3::new(4.0, 4.0, 0.0),
                                border_radius: None,
                            })
                            .insert(Bullet {
                                timer: Timer::from_seconds(4.0, false),
                            });
                    });
                    let cloned = weapon.clone();
                    let string_handle = weapon_enum_to_string(cloned.asset);
                    let texture: Handle<Image> = asset_server.load(&string_handle);
                    commands.entity(entity).despawn_recursive();
                    commands
                        .spawn_bundle(SpriteBundle {
                            texture,
                            transform: Transform::from_translation(player.location),
                            ..default()
                        })
                        .insert(HeldItem)
                        .insert(Weapon {
                            asset: cloned.asset,
                        });
                })
            });
        }
        None => {
            return;
        } // do nothing
    }
}

#[derive(Component)]
pub struct Bullet {
    timer: Timer,
}

#[derive(Component)]
pub struct Spinning {
    last_angle: f32,
}

pub fn spin_spinners(mut spinners: Query<(&mut Transform, &mut Spinning), With<Spinning>>) {}

pub fn fire_weapon(
    mut commands: Commands,
    query_held_item: Query<(&mut Transform, Entity, &Weapon), With<HeldItem>>,
    mut player_query: Query<(&mut Player, &mut Velocity), With<Player>>,
    asset_server: Res<AssetServer>,
    buttons: Res<Input<MouseButton>>,
) {
    let player_check = player_query.iter_mut().next();
    match player_check {
        Some((player, mut player_vel)) => {
            if buttons.just_pressed(MouseButton::Left) {
                query_held_item
                    .iter()
                    .for_each(|(held_trans, held_item, weapon)| {
                        commands.entity(held_item).despawn_recursive();
                        let spent_weapon = weapon_enum_to_string(weapon.asset);
                        let spent_asset: Handle<Image> = asset_server.load(&spent_weapon);
                        commands
                            .spawn_bundle(SpriteBundle {
                                texture: spent_asset,
                                transform: Transform::from_translation(held_trans.translation),
                                ..default()
                            })
                            .insert(RigidBody::Dynamic)
                            .insert(CollisionShape::Cuboid {
                                half_extends: Vec3::new(4.0, 4.0, 0.0),
                                border_radius: None,
                            })
                            .insert(Spinning { last_angle: 0. })
                            .insert(Collisions::default())
                            .insert(
                                CollisionLayers::none()
                                    .with_group(Layers::Projectiles)
                                    .with_mask(Layers::World),
                            )
                            .insert(Bullet {
                                timer: Timer::from_seconds(4.0, false),
                            });
                        let bullet: Handle<Image> = asset_server.load("images/Bullet.png");
                        let looking_at = player.looking_at;
                        let bullet_speed = 500.0;
                        commands
                            .spawn_bundle(SpriteBundle {
                                texture: bullet,
                                transform: Transform {
                                    translation: held_trans.translation,
                                    rotation: Quat::from_rotation_z(looking_at),
                                    ..default()
                                },
                                ..default()
                            })
                            .insert(RigidBody::Dynamic)
                            .insert(
                                CollisionLayers::none()
                                    .with_group(Layers::Projectiles)
                                    .with_mask(Layers::World)
                                    .with_mask(Layers::Enemies),
                            )
                            .insert(Velocity {
                                linear: Vec3::new(
                                    cosf(looking_at) * bullet_speed,
                                    sinf(looking_at) * bullet_speed,
                                    0.0,
                                ),
                                ..default()
                            })
                            .insert(Bullet {
                                timer: Timer::from_seconds(5.0, false),
                            });
                        player_vel.linear =
                            Vec3::new(-cosf(looking_at) * 100.0, -sinf(looking_at) * 100.0, 0.);
                    });
            }
        }
        None => {} // do nothing
    }
}

#[derive(PhysicsLayer)]
enum Layers {
    World,
    Player,
    Enemies,
    Weapons,
    Projectiles,
}

pub struct WeaponSpawns {
    timer: Timer,
}

#[derive(Component)]
pub struct Player {
    jump_height: f32,
    location: Vec3,
    looking_at: f32,
}

#[derive(Component)]
pub struct MyCamera;

pub fn setup_camera(mut commands: Commands) {
    commands
        .spawn_bundle(Camera2dBundle {
            projection: OrthographicProjection {
                scale: 250.0,
                scaling_mode: ScalingMode::FixedVertical(1.),
                ..default()
            },
            ..default()
        })
        .insert(MyCamera);
}

#[derive(Component)]
pub struct HeldItem;

pub fn create_floor(mut commands: Commands, asset_server: Res<AssetServer>) {
    let floor_sprite: Handle<Image> = asset_server.load("images/Floor.png");
    let floor_size = Vec2::new(28.0, 28.0);
    for i in 0..20 {
        commands
            .spawn_bundle(SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(floor_size),
                    ..default()
                },
                texture: floor_sprite.clone(),
                transform: Transform::from_translation(Vec3::new(
                    i as f32 * 28.0 + -240.0,
                    -120.0,
                    0.0,
                )),
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
                    .with_mask(Layers::Player)
                    .with_mask(Layers::Projectiles),
            );
    }
}

#[derive(Component)]
pub struct Dashing {
    timer: Timer,
    direction: Directions,
}

impl Default for Dashing {
    fn default() -> Self {
        Dashing {
            timer: Timer::from_seconds(0.1, false),
            direction: Directions::Left,
        }
    }
}

pub fn create_scoreboard(mut commads: Commands, score: Res<Score>) {
    let score_string = String::from("Score: 0");
    commads
        .spawn_bundle(NodeBundle {
            style: Style {
                display: Display::Flex,
                flex_direction: FlexDirection::ColumnReverse,
                size: Size {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ..default()
            },
            ..default()
        })
        .with_children(|ui_parent| {
            ui_parent.spawn_bundle(TextBundle {
                text: Text {
                    sections: vec![],
                    ..default()
                },
                style: Style {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    size: Size {
                        width: Val::Percent(100.0),
                        height: Val::Percent(20.0),
                        ..default()
                    },
                    ..default()
                },
                ..default()
            });
        });
}

pub fn create_character(mut commands: Commands, asset_server: Res<AssetServer>) {
    let character_sprite: Handle<Image> = asset_server.load("images/Character.png");
    let sprite_size = Vec2::new(12.0, 28.0);
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(sprite_size),
                ..Default::default()
            },
            texture: character_sprite.clone(),
            transform: Transform::from_translation(Vec3::new(0., -92., 1.0)),
            ..default()
        })
        .insert(RigidBody::Dynamic)
        .insert(CollisionShape::Cuboid {
            half_extends: sprite_size.extend(0.0) / 2.0,
            border_radius: None,
        })
        .insert(Player {
            jump_height: 100.0,
            location: Vec3::new(0., 92., 1.),
            looking_at: 0.0,
        })
        .insert(Velocity { ..default() })
        .insert(
            CollisionLayers::none()
                .with_group(Layers::Player)
                .with_mask(Layers::World)
                .with_mask(Layers::Weapons),
        );
}

pub fn move_player(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    mut player_query: Query<(&mut Transform, &mut Velocity, &mut Player, Entity), Without<Dashing>>,
    mut dashers: Query<(&mut Transform, &mut Player, &Dashing, &mut Velocity), With<Dashing>>,
    mut dash_time: ResMut<DashTimer>,
    time: Res<Time>,
) {
    let player_check = player_query.iter_mut().next();
    match player_check {
        Some((mut trans, mut velocity, mut player, entity)) => {
            trans.rotation = Quat::from_rotation_z(0.0);
            if keys.just_pressed(KeyCode::D) {
                if dash_time.timer.finished() || dash_time.direction != Directions::Right {
                    dash_time.timer = Timer::from_seconds(0.2, false);
                    dash_time.direction = Directions::Right;
                    if velocity.linear.x >= -50. {
                        trans.translation.x += 1.0;
                    }
                } else {
                    //trans.translation.x += 20.0;
                    dash_time.timer.set_elapsed(Duration::from_secs(50));
                    commands.entity(entity).insert(Dashing {
                        direction: Directions::Right,
                        ..default()
                    });
                }
            } else if keys.pressed(KeyCode::D) {
                if velocity.linear.x >= -50. {
                    trans.translation.x += 1.0;
                }
            }
            if keys.just_pressed(KeyCode::A) {
                if dash_time.timer.finished() || dash_time.direction != Directions::Left {
                    dash_time.timer = Timer::from_seconds(0.2, false);
                    dash_time.direction = Directions::Left;
                    if velocity.linear.x <= 50. {
                        trans.translation.x += -1.0;
                    }
                } else {
                    //trans.translation.x += -20.0;
                    dash_time.timer.set_elapsed(Duration::from_secs(50));
                    commands.entity(entity).insert(Dashing::default());
                }
            } else if keys.pressed(KeyCode::A) {
                if velocity.linear.x <= 50. {
                    trans.translation.x += -1.0;
                }
            }
            if keys.just_pressed(KeyCode::Space) {
                velocity.linear.y = player.jump_height;
            }
            player.location = trans.translation;
        }
        _default => {} // do nothing
    }
    dashers
        .iter_mut()
        .for_each(|(mut dasher_trans, mut dasher, dashing, mut vel)| {
            vel.linear = Vec3::splat(0.);
            match dashing.direction {
                Directions::Left => {
                    dasher_trans.translation.x -= 250.0 * time.delta_seconds();
                }
                Directions::Right => {
                    dasher_trans.translation.x += 250.0 * time.delta_seconds();
                }
            }
            dasher.location = dasher_trans.translation;
        });
}

fn point_held_item(
    wnds: Res<Windows>,
    q_camera: Query<(&Camera, &GlobalTransform), With<MyCamera>>,
    mut players: Query<&mut Player>,
    mut held_items: Query<&mut Transform, With<HeldItem>>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let (camera, camera_transform) = q_camera.single();

    let wnd = if let RenderTarget::Window(id) = camera.target {
        wnds.get(id).unwrap()
    } else {
        wnds.get_primary().unwrap()
    };

    if let Some(screen_pos) = wnd.cursor_position() {
        let player_maybe = players.iter_mut().next();
        let held_item_maybe = held_items.iter_mut().next();
        match player_maybe {
            Some(mut player) => {
                match held_item_maybe {
                    // get the size of the window
                    Some(mut held_item) => {
                        let window_size = Vec2::new(wnd.width() as f32, wnd.height() as f32);

                        // convert screen position [0..resolution] to ndc [-1..1] (gpu coordinates)
                        let ndc = (screen_pos / window_size) * 2.0 - Vec2::ONE;

                        // matrix for undoing the projection and camera transform
                        let ndc_to_world = camera_transform.compute_matrix()
                            * camera.projection_matrix().inverse();

                        // use it to convert ndc to world-space coordinates
                        let world_pos = ndc_to_world.project_point3(ndc.extend(-1.0));

                        // reduce it to a 2D value
                        let world_pos: Vec2 = world_pos.truncate();
                        let player_to_cursor = Vec2::new(
                            world_pos.x - player.location.x,
                            world_pos.y - player.location.y,
                        );
                        let angle = atan2f(player_to_cursor.y, player_to_cursor.x);
                        player.looking_at = angle;
                        let player_cursor_distance =
                            (player_to_cursor.x.powi(2) + player_to_cursor.y.powi(2)).sqrt();
                        let distance_from_center = if player_cursor_distance > 5.0 {
                            5.0
                        } else {
                            player_cursor_distance
                        };
                        let item_pos = Vec3::new(
                            player.location.x + cosf(angle) * distance_from_center,
                            player.location.y + sinf(angle) * distance_from_center,
                            10.0,
                        );
                        held_item.translation = item_pos;
                    }
                    _default => {
                        // none found
                    }
                }
            }
            _default => {
                // do nothing
            }
        }
    }
}
