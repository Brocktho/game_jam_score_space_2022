use std::time::Duration;

use bevy::prelude::{Camera2dBundle, *};
use bevy::render::camera::{RenderTarget, ScalingMode};
use bevy_inspector_egui::{Inspectable, InspectorPlugin, WorldInspectorPlugin};
use heron::{prelude::*, PhysicsSteps};
use libm::{atan2f, cosf, sinf};
use math::round;
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

pub struct DifficultyTimer {
    difficulty: i64,
    timer: Timer,
}

pub struct EnemyTimer {
    timer: Timer,
}

/* pub struct ConfigSettings {
    sfx: f32,
    music: f32,

} */

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(PhysicsPlugin::default())
        .insert_resource(DifficultyTimer {
            difficulty: 1,
            timer: Timer::from_seconds(5.0, true),
        })
        .insert_resource(EnemyTimer {
            timer: Timer::from_seconds(2.0, true),
        })
        .insert_resource(Gravity::from(Vec3::new(0.0, -70.1, 0.0)))
        .insert_resource(PhysicsTime::new(1.))
        .insert_resource(PhysicsSteps::from_steps_per_seconds(30.))
        .insert_resource(WeaponSpawns {
            timer: Timer::from_seconds(1.0, true),
        })
        .insert_resource(Score::default())
        .add_system(fire_weapon)
        //.add_plugin(WorldInspectorPlugin::new())
        .insert_resource(DashTimer {
            timer: Timer::from_seconds(0.0001, false),
            direction: Directions::Left,
        })
        .add_startup_system(create_character)
        .add_startup_system(create_scoreboard)
        .add_startup_system(setup_camera)
        .add_system(tick_timers)
        .add_system(move_player)
        .add_startup_system(create_borders)
        .add_system(grab_weapon)
        .add_system(point_held_item)
        .add_system(update_score)
        .add_system(spin_spinners)
        .add_system(move_enemies)
        .add_system(spawn_warned)
        .add_system(animate_sprites)
        .add_system(spawn_warned_enemy)
        .add_system(handle_slides)
        .add_system(handle_jumpers)
        .add_system(handle_bullet_collision)
        .add_system(handle_despawner)
        .add_system(handle_shooter)
        .add_startup_system(play_music)
        .run();
}

pub fn handle_bullet_collision(mut commands: Commands, bullets: Query<&Collisions, With<Bullet>>) {
    bullets.iter().for_each(|collision| {
        collision.entities().for_each(|entity| {
            commands.entity(entity).log_components();
            commands.entity(entity).despawn_recursive();
        });
    });
}

pub fn play_music(audio: Res<Audio>, asset_server: Res<AssetServer>) {
    audio.play_with_settings(
        asset_server.load("sounds/backtrack.ogg"),
        PlaybackSettings {
            repeat: true,
            volume: 0.1,
            speed: 1.0,
        },
    );
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

pub fn handle_difficulty(
    mut enemy_timer: ResMut<EnemyTimer>,
    mut difficulty: ResMut<DifficultyTimer>,
) {
    let old_dur = enemy_timer.timer.duration();
    enemy_timer.timer.set_duration(Duration::from_secs_f32(
        if old_dur.as_secs_f32() - 0.1 > 0.1 {
            old_dur.as_secs_f32() - 0.1
        } else {
            0.1
        },
    ));
    difficulty.difficulty += 1;
    if difficulty.difficulty >= 25 {
        let old_diff = difficulty.timer.duration();
        difficulty.timer.set_duration(Duration::from_secs_f32(
            if old_diff.as_secs_f32() - 0.1 > 0.1 {
                old_diff.as_secs_f32() - 0.1
            } else {
                0.1
            },
        ));
    }
}

#[derive(Copy, Clone)]
pub enum Behavior {
    Walker,
    Jumper,
    Shooter,
    BurstShooter,
}

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(Timer);

#[derive(Component)]
pub struct Enemy {
    asset: Behavior,
    health: i8,
    direction: f32,
    delay_move: Timer,
}

#[derive(Component)]
pub struct SpawnEnemy {
    asset: Behavior,
    location: Vec3,
    timer: Timer,
}

pub fn behavior_to_asset(behav: Behavior) -> String {
    match behav {
        Behavior::Walker => String::from("images/BaseEnemy.png"),
        Behavior::Jumper => String::from("images/Jumper.png"),
        Behavior::Shooter => String::from("images/Shooter.png"),
        Behavior::BurstShooter => String::from("images/BurstShooter.png"),
    }
}

pub enum Sounds {
    PlayerJump,
    EnemyJump,
    GunShot,
    SniperShot,
    ShotgunShot,
    Rocket,
    Rock,
    Airplane,
    EnemyShot,
}

#[derive(Component)]
pub struct Jump {
    timer: Timer,
    audio: Sounds,
}

#[derive(Component)]
pub struct Slide {
    timer: Timer,
}

#[derive(Component)]
pub struct BurstShot {
    timer: Timer,
    audio: Sounds,
}

#[derive(Component)]
pub struct Shooter {
    timer: Timer,
    audio: Sounds,
}

#[derive(Component)]
pub struct Despawner(Timer);

pub fn handle_despawner(
    mut commands: Commands,
    mut despawners: Query<(&mut Despawner, Entity), With<Despawner>>,
    time: Res<Time>,
) {
    despawners.iter_mut().for_each(|(mut despawn, entity)| {
        despawn.0.tick(time.delta());
        if despawn.0.finished() {
            commands.entity(entity).despawn_recursive();
        }
    });
}

pub fn handle_shooter(
    mut commands: Commands,
    mut shooters: Query<(&Transform, &mut Shooter, Entity), With<Shooter>>,
    player: Query<&Player>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    player.iter().for_each(|player| {
        shooters
            .iter_mut()
            .for_each(|(trans, mut shooter, entity)| {
                shooter.timer.tick(time.delta());
                let dx = trans.translation.x - player.location.x;
                let dy = trans.translation.y - player.location.y;
                let angle = atan2f(dy, dx);
                if shooter.timer.finished() {
                    let bullet_handle: Handle<Image> = asset_server.load("images/EnemyBullet.png");
                    let texture_atlas =
                        TextureAtlas::from_grid(bullet_handle, Vec2::new(3.0, 3.0), 4, 2);
                    let sprite = texture_atlases.add(texture_atlas);
                    commands
                        .spawn_bundle(SpriteSheetBundle {
                            transform: Transform::from_translation(trans.translation),
                            texture_atlas: sprite,
                            ..default()
                        })
                        .insert(AnimationTimer(Timer::from_seconds(0.2, true)))
                        .insert(Collisions::default())
                        .insert(RigidBody::Sensor)
                        .insert(CollisionShape::Sphere { radius: 1.5 })
                        .insert(Velocity {
                            linear: Vec3::new(-sinf(angle) * 50.0, cosf(angle) * 50.0, 0.0),
                            ..default()
                        });
                    commands.entity(entity).remove::<Shooter>();
                }
                let trace_handle: Handle<Image> = asset_server.load("images/Trace.png");
                let sprite_size = Vec2::new(500., 1.0);
                commands
                    .spawn_bundle(SpriteBundle {
                        texture: trace_handle,
                        sprite: Sprite {
                            custom_size: Some(sprite_size),
                            ..default()
                        },
                        transform: Transform {
                            translation: trans.translation,
                            rotation: Quat::from_rotation_z(angle),
                            ..default()
                        },
                        ..default()
                    })
                    .insert(Despawner(Timer::from_seconds(0.05, false)));
            });
    });
}

pub fn handle_slides(
    mut commands: Commands,
    mut sliders: Query<(&mut Transform, &mut Slide, Entity, &Enemy), With<Slide>>,
    time: Res<Time>,
) {
    sliders
        .iter_mut()
        .for_each(|(mut trans, mut slide, entity, enemy)| {
            slide.timer.tick(time.delta());
            if slide.timer.finished() {
                trans.scale.x = 1.0;
                commands.entity(entity).remove::<Slide>();
                return;
            }
            trans.translation.x += 20.0 * time.delta_seconds() * enemy.direction;
            trans.scale.x += 0.5 * time.delta_seconds();
        });
}

pub fn handle_jumpers(
    mut commands: Commands,
    mut jumpers: Query<(&mut Transform, &mut Jump, &mut Velocity, Entity, &mut Enemy), With<Jump>>,
    time: Res<Time>,
) {
    jumpers
        .iter_mut()
        .for_each(|(mut trans, mut jump, mut vel, entity, mut enemy)| {
            jump.timer.tick(time.delta());
            if jump.timer.finished() {
                trans.scale.y = 1.0;
                enemy.delay_move.reset();
                let mut rand = rand::thread_rng();
                let x_vel = rand.gen_range(20.0..100.0) as f32;
                let y_vel = rand.gen_range(200.0..500.0) as f32;
                let direction = round::floor(rand.gen_range(-1.0..1.0), -1) as f32;
                vel.linear.y = y_vel;
                vel.linear.x = x_vel * direction;
                commands.entity(entity).remove::<Jump>();
                return;
            }
            trans.scale.y -= 0.3 * time.delta_seconds();
        });
}

pub fn move_enemies(
    mut commands: Commands,
    mut enemies: Query<
        (&mut Enemy, Entity),
        (
            With<Enemy>,
            Without<Jump>,
            Without<Slide>,
            Without<Shooter>,
            Without<BurstShot>,
        ),
    >,
    time: Res<Time>,
) {
    enemies.iter_mut().for_each(|(mut enemy, entity)| {
        enemy.delay_move.tick(time.delta());
        if enemy.delay_move.finished() {
            match enemy.asset {
                Behavior::Walker => commands.entity(entity).insert(Slide {
                    timer: Timer::from_seconds(0.5, false),
                }),
                Behavior::BurstShooter => commands.entity(entity).insert(BurstShot {
                    timer: Timer::from_seconds(1.4, false),
                    audio: Sounds::EnemyShot,
                }),
                Behavior::Jumper => commands.entity(entity).insert(Jump {
                    timer: Timer::from_seconds(2.5, false),
                    audio: Sounds::EnemyJump,
                }),
                Behavior::Shooter => commands.entity(entity).insert(Shooter {
                    timer: Timer::from_seconds(1.0, false),
                    audio: Sounds::EnemyShot,
                }),
            };
        }
    });
}

/* pub fn spawn_enemy(commands: &mut Commands, asset_server: AssetServer, behavior: Behavior) {
    let texture: Handle<Image> = asset_server.load("images/BaseEnemy.png");
    commands
        .spawn_bundle(SpriteBundle {
            texture,
            transform: Transform::from_translation(Vec3::new(-180., -90., 0.)),
            sprite: Sprite {
                custom_size: Some(Vec2::new(20.0, 20.0)),
                ..default()
            },
            ..default()
        })
        .insert(Enemy {
            asset: behavior,
            health: 1,
            direction: 1.0,
        })
        .insert(CollisionShape::Cuboid {
            border_radius: None,
            half_extends: Vec3::new(20.0, 20.0, 0.0),
        })
        .insert(RigidBody::Dynamic)
        .insert(
            CollisionLayers::none()
                .with_group(Layers::Enemies)
                .with_mask(Layers::World)
                .with_mask(Layers::Player)
                .with_mask(Layers::Projectiles),
        )
        .insert(Collisions::default());
}
 */
pub fn animate_sprites(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut sprites: Query<(
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
        &Handle<TextureAtlas>,
    )>,
) {
    for (mut timer, mut sprite, texture_atlas_handle) in &mut sprites {
        timer.tick(time.delta());
        if timer.just_finished() {
            let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
            sprite.index = (sprite.index + 1) % texture_atlas.textures.len();
        }
    }
}

pub fn spawn_warned(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut waiting_weapons: Query<(&mut SpawnWeapon, Entity), With<SpawnWeapon>>,
    time: Res<Time>,
) {
    waiting_weapons.iter_mut().for_each(|(mut weapon, entity)| {
        weapon.timer.tick(time.delta());
        if weapon.timer.finished() {
            commands.entity(entity).despawn_recursive();
            let weapon_string = weapon_enum_to_string(weapon.asset);
            let weapon_sprite: Handle<Image> = asset_server.load(&weapon_string);
            let weapon_size = Vec2::new(14., 4.);
            commands
                .spawn_bundle(SpriteBundle {
                    transform: Transform::from_translation(weapon.position),
                    sprite: Sprite {
                        color: Color::Rgba {
                            red: 0.0,
                            green: 0.0,
                            blue: 0.0,
                            alpha: 0.0,
                        },
                        custom_size: Some(weapon_size),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent
                        .spawn_bundle(SpriteBundle {
                            texture: weapon_sprite,
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
                .insert(CollisionLayers::none().with_group(Layers::Weapons))
                .insert(Bullet {
                    timer: Timer::from_seconds(5.0, false),
                })
                .insert(Name::new("Weapon"));
        }
    });
}

pub fn spawn_warned_enemy(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut waiting_weapons: Query<(&mut SpawnEnemy, Entity), With<SpawnEnemy>>,
    time: Res<Time>,
) {
    // poor naming here, thanks copy paste :)
    waiting_weapons.iter_mut().for_each(|(mut weapon, entity)| {
        weapon.timer.tick(time.delta());
        if weapon.timer.finished() {
            commands.entity(entity).despawn_recursive();
            let weapon_string = behavior_to_asset(weapon.asset.clone());
            let weapon_sprite: Handle<Image> = asset_server.load(&weapon_string);
            let texture_atlas = TextureAtlas::from_grid(weapon_sprite, Vec2::new(15., 15.), 8, 4);
            let sprite = texture_atlases.add(texture_atlas);
            commands
                .spawn_bundle(SpriteSheetBundle {
                    transform: Transform::from_translation(weapon.location),
                    texture_atlas: sprite.clone(),
                    ..default()
                })
                .insert(CollisionShape::Cuboid {
                    border_radius: None,
                    half_extends: Vec3::new(7.5, 7.5, 0.0),
                })
                .insert(RigidBody::Dynamic)
                .insert(
                    CollisionLayers::none()
                        .with_group(Layers::Enemies)
                        .with_mask(Layers::World)
                        .with_mask(Layers::Player)
                        .with_mask(Layers::Projectiles),
                )
                .insert(AnimationTimer(Timer::from_seconds(0.055, true)))
                .insert(Collisions::default())
                .insert(Enemy {
                    asset: weapon.asset,
                    health: 1,
                    direction: 1.0,
                    delay_move: Timer::from_seconds(
                        match weapon.asset {
                            Behavior::Jumper => 2.0,
                            _default => 1.0,
                        },
                        true,
                    ),
                })
                .insert(Velocity::default())
                .insert(Name::new("Enemy"));
        }
    });
}

pub fn spawn_enemy_warning(
    commands: &mut Commands,
    texture: Handle<Image>,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    behavior: Behavior,
) {
    let mut random = rand::thread_rng();
    let random_x = random.gen_range(-200.0..200.0) as f32;
    let texture_atlas = TextureAtlas::from_grid(texture, Vec2::new(5.0, 5.0), 5, 3);
    let sprite = texture_atlases.add(texture_atlas);
    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: sprite,
            transform: Transform {
                translation: Vec3::new(random_x, -92.0, 0.0),
                scale: Vec3::splat(2.0),
                ..default()
            },
            ..default()
        })
        .insert(AnimationTimer(Timer::from_seconds(0.066, true)))
        .insert(SpawnEnemy {
            timer: Timer::from_seconds(1.0, false),
            asset: behavior,
            location: Vec3::new(random_x, -92.0, 0.0),
        })
        .insert(Name::new("Weapon Warning"));
}

pub fn tick_timers(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut weapon_time: ResMut<WeaponSpawns>,
    mut dash_time: ResMut<DashTimer>,
    mut any_dashing: Query<(Entity, &mut Dashing), With<Dashing>>,
    mut any_bullets: Query<(Entity, &mut Bullet)>,
    time: Res<Time>,
    mut any_spinners: Query<&mut Spinning>,
    mut enemy_timer: ResMut<EnemyTimer>,
    mut difficulty: ResMut<DifficultyTimer>,
    enemies: Query<&Enemy>,
) {
    difficulty.timer.tick(time.delta());
    enemy_timer.timer.tick(time.delta());
    if enemy_timer.timer.finished() {
        if enemies.iter().len() <= 100 as usize {
            for _i in 0..if difficulty.difficulty < 8 {
                difficulty.difficulty
            } else {
                6
            } {
                let sheet: Handle<Image> = asset_server.load("images/SpawnEnemy.png");
                let mut rand = rand::thread_rng();
                let decider = rand.gen_range(0..difficulty.difficulty);
                let spawned_type = match decider % 12 {
                    0 => Behavior::Walker,
                    1 => Behavior::Jumper,
                    2 => Behavior::Shooter,
                    3 => Behavior::BurstShooter,
                    4 => Behavior::Jumper,
                    5 => Behavior::Jumper,
                    6 => Behavior::Shooter,
                    7 => Behavior::Jumper,
                    8 => Behavior::BurstShooter,
                    9 => Behavior::Walker,
                    10 => Behavior::Shooter,
                    11 => Behavior::Shooter,
                    _def => Behavior::Jumper,
                };
                spawn_enemy_warning(&mut commands, sheet, &mut texture_atlases, spawned_type);
                //spawn_enemy(&mut commands, asset_server.clone(), Behavior::Walker);
            }
        }
    }
    if difficulty.timer.finished() {
        handle_difficulty(enemy_timer, difficulty);
    }
    any_spinners.iter_mut().for_each(|mut spinner| {
        spinner.timer.tick(time.delta());
    });
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
        let warn_sprite: Handle<Image> = asset_server.load("images/SpawnWeapon.png");
        warn_weapon_spawn(commands, warn_sprite, texture_atlases, Weapons::Base);
    }
}

pub fn hurt_player() {}

#[derive(Component)]
pub struct SpawnWeapon {
    timer: Timer,
    asset: Weapons,
    position: Vec3,
}

pub fn warn_weapon_spawn(
    mut commands: Commands,
    image: Handle<Image>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    weapon: Weapons,
) {
    let mut random = rand::thread_rng();
    let random_x = random.gen_range(-200.0..200.0) as f32;
    let texture_atlas = TextureAtlas::from_grid(image, Vec2::new(4.0, 4.0), 3, 6);
    let sprite = texture_atlases.add(texture_atlas);
    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: sprite,
            transform: Transform {
                translation: Vec3::new(random_x, 120.0, 0.0),
                scale: Vec3::splat(2.0),
                ..default()
            },
            ..default()
        })
        .insert(AnimationTimer(Timer::from_seconds(0.055, true)))
        .insert(SpawnWeapon {
            timer: Timer::from_seconds(1.0, false),
            asset: weapon,
            position: Vec3::new(random_x, 120.0, 0.0),
        })
        .insert(Name::new("Weapon Warning"));
}

pub fn grab_weapon(
    mut commands: Commands,
    weapons: Query<(Entity, &Collisions, &Weapon), With<Weapon>>,
    player_query: Query<&Player>,
    asset_server: Res<AssetServer>,
    query_held_item: Query<(Entity, &Weapon), With<HeldItem>>,
    mut score: ResMut<Score>,
    difficulty: Res<DifficultyTimer>,
) {
    let player_check = player_query.iter().next();
    match player_check {
        Some(player) => {
            weapons.iter().for_each(|(entity, collisions, &weapon)| {
                collisions.entities().for_each(|_collision| {
                    score.score += 2 * difficulty.difficulty;
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
                            })
                            .insert(Name::new("Spent Weapon"))
                            .insert(
                                CollisionLayers::none()
                                    .with_group(Layers::Projectiles)
                                    .with_mask(Layers::World),
                            );
                    });
                    let cloned = weapon.clone();
                    let string_handle = weapon_enum_to_string(cloned.asset);
                    let texture: Handle<Image> = asset_server.load(&string_handle);
                    commands.entity(entity).despawn_recursive();
                    commands
                        .spawn_bundle(SpriteBundle {
                            sprite: Sprite {
                                flip_x: true,
                                ..default()
                            },
                            texture,
                            transform: Transform::from_translation(player.location),
                            ..default()
                        })
                        .insert(HeldItem)
                        .insert(Weapon {
                            asset: cloned.asset,
                        })
                        .insert(Name::new("Held Item"));
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
    timer: Timer,
}

pub fn spin_spinners(mut spinners: Query<(&mut Transform, &mut Spinning), With<Spinning>>) {
    spinners.iter_mut().for_each(|(mut trans, mut spin)| {
        if spin.timer.finished() {
            return;
        }
        spin.last_angle += 0.1;
        trans.rotation = Quat::from_rotation_z(spin.last_angle);
    });
}

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
                        let mut rand = rand::thread_rng();
                        let random_x = rand.gen_range(30.0..100.0) as f32;
                        let random_y = rand.gen_range(30.0..100.0) as f32;
                        commands.entity(held_item).despawn_recursive();
                        let spent_weapon = weapon_enum_to_string(weapon.asset);
                        let spent_asset: Handle<Image> = asset_server.load(&spent_weapon);
                        let looking_at = player.looking_at;

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
                            .insert(Spinning {
                                last_angle: 0.,
                                timer: Timer::from_seconds(1.0, false),
                            })
                            .insert(
                                CollisionLayers::none()
                                    .with_group(Layers::Projectiles)
                                    .with_mask(Layers::World),
                            )
                            .insert(Bullet {
                                timer: Timer::from_seconds(4.0, false),
                            })
                            .insert(Name::new("Spent spinning gun"))
                            .insert(Velocity {
                                linear: Vec3::new(
                                    -cosf(looking_at) * random_x,
                                    -sinf(looking_at) * random_y,
                                    0.0,
                                ),
                                ..default()
                            });
                        let bullet: Handle<Image> = asset_server.load("images/Bullet.png");
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
                            .insert(CollisionShape::Cuboid {
                                half_extends: Vec3::new(2.0, 2.0, 1.0),
                                border_radius: None,
                            })
                            .insert(RigidBody::Dynamic)
                            .insert(
                                CollisionLayers::none()
                                    .with_group(Layers::Projectiles)
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
                            })
                            .insert(Collisions::default())
                            .insert(Name::new("bullet"));
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
        .insert(MyCamera)
        .insert(Name::new("Camera"));
}

#[derive(Component)]
pub struct HeldItem;

pub fn create_borders(mut commands: Commands, asset_server: Res<AssetServer>) {
    let floor_sprite: Handle<Image> = asset_server.load("images/Floor.png");
    let floor_size = Vec2::new(28.0, 28.0);
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::Rgba {
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 0.0,
                },
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for i in 0..20 {
                let mut block_name = String::from("Block ");
                block_name += &(i.to_string());
                parent
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
                            .with_mask(Layers::Projectiles)
                            .with_mask(Layers::Enemies),
                    )
                    .insert(Name::new(block_name));
            }
        });
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::Rgba {
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 0.0,
                },
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for i in 0..9 {
                let mut block_name = String::from("Block ");
                block_name += &(i.to_string());
                parent
                    .spawn_bundle(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(floor_size),
                            ..default()
                        },
                        texture: floor_sprite.clone(),
                        transform: Transform::from_translation(Vec3::new(
                            -235.0,
                            i as f32 * 28.0 + -92.0,
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
                            .with_mask(Layers::Projectiles)
                            .with_mask(Layers::Enemies),
                    )
                    .insert(Name::new(block_name));
            }
        });
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::Rgba {
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 0.0,
                },
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for i in 0..9 {
                let mut block_name = String::from("Block ");
                block_name += &(i.to_string());
                parent
                    .spawn_bundle(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(floor_size),
                            ..default()
                        },
                        texture: floor_sprite.clone(),
                        transform: Transform::from_translation(Vec3::new(
                            235.0,
                            28.0 * i as f32 + -92.0,
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
                            .with_mask(Layers::Projectiles)
                            .with_mask(Layers::Enemies),
                    )
                    .insert(Name::new(block_name));
            }
        });
    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                color: Color::Rgba {
                    red: 0.0,
                    green: 0.0,
                    blue: 0.0,
                    alpha: 0.0,
                },
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for i in 0..20 {
                let mut block_name = String::from("Block ");
                block_name += &(i.to_string());
                parent
                    .spawn_bundle(SpriteBundle {
                        sprite: Sprite {
                            custom_size: Some(floor_size),
                            ..default()
                        },
                        texture: floor_sprite.clone(),
                        transform: Transform::from_translation(Vec3::new(
                            i as f32 * 28.0 + -240.0,
                            138.0,
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
                            .with_mask(Layers::Projectiles)
                            .with_mask(Layers::Enemies),
                    )
                    .insert(Name::new(block_name));
            }
        });
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

pub fn create_scoreboard(mut commads: Commands, asset_server: Res<AssetServer>) {
    let font_handle: Handle<Font> = asset_server.load("fonts/RobotoMono.ttf");
    commads
        .spawn_bundle(NodeBundle {
            color: UiColor(Color::Rgba {
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                alpha: 0.0,
            }),
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
        .insert(Name::new("UI Background"))
        .with_children(|ui_parent| {
            ui_parent
                .spawn_bundle(NodeBundle {
                    color: UiColor(Color::Rgba {
                        red: 0.0,
                        green: 0.0,
                        blue: 0.0,
                        alpha: 0.0,
                    }),
                    style: Style {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_content: AlignContent::Center,
                        size: Size {
                            width: Val::Percent(100.0),
                            height: Val::Percent(10.0),
                            ..default()
                        },
                        ..default()
                    },
                    ..default()
                })
                .insert(ScoreParent)
                .insert(Name::new("Score Block"))
                .with_children(|score_parent| {
                    score_parent.spawn_bundle(TextBundle {
                        text: Text {
                            sections: vec![TextSection {
                                value: String::from("Score: "),
                                style: TextStyle {
                                    font: font_handle.clone(),
                                    font_size: 40.,
                                    ..default()
                                },
                            }],
                            ..default()
                        },
                        ..default()
                    });
                    score_parent
                        .spawn_bundle(TextBundle {
                            text: Text {
                                sections: vec![TextSection {
                                    value: String::from("0"),
                                    style: TextStyle {
                                        font: font_handle.clone(),
                                        font_size: 40.,
                                        ..default()
                                    },
                                }],
                                ..default()
                            },
                            ..default()
                        })
                        .insert(ScoreLabel);
                });
        });
}

pub fn update_score(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    score: Res<Score>,
    existing_score: Query<Entity, With<ScoreLabel>>,
    score_parent: Query<Entity, With<ScoreParent>>,
) {
    if !score.is_changed() {
        return;
    }
    let font_handle: Handle<Font> = asset_server.load("fonts/RobotoMono.ttf");

    existing_score.iter().for_each(|score| {
        commands.entity(score).despawn_recursive();
    });
    score_parent.iter().for_each(|parent| {
        commands.entity(parent).add_children(|builder| {
            let mut new_score = String::new();
            new_score += &score.score.to_string();
            builder
                .spawn_bundle(TextBundle {
                    text: Text {
                        sections: vec![TextSection {
                            value: new_score,
                            style: TextStyle {
                                font: font_handle.clone(),
                                font_size: 40.,
                                ..default()
                            },
                        }],
                        ..default()
                    },
                    ..default()
                })
                .insert(ScoreLabel);
        });
    });
}
#[derive(Component)]
pub struct ScoreLabel;

#[derive(Component)]
pub struct ScoreParent;

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
                .with_mask(Layers::Weapons)
                .with_mask(Layers::Enemies),
        )
        .insert(Name::new("Player"))
        .insert(Collisions::default());
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
                        velocity.linear.x = 0.;
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
                    velocity.linear.x = 0.;
                }
            }
            if keys.just_pressed(KeyCode::A) {
                if dash_time.timer.finished() || dash_time.direction != Directions::Left {
                    dash_time.timer = Timer::from_seconds(0.2, false);
                    dash_time.direction = Directions::Left;
                    if velocity.linear.x <= 50. {
                        trans.translation.x += -1.0;
                        velocity.linear.x = 0.;
                    }
                } else {
                    //trans.translation.x += -20.0;
                    dash_time.timer.set_elapsed(Duration::from_secs(50));
                    commands.entity(entity).insert(Dashing::default());
                }
            } else if keys.pressed(KeyCode::A) {
                if velocity.linear.x <= 50. {
                    trans.translation.x += -1.0;
                    velocity.linear.x = 0.;
                }
            }
            if keys.just_pressed(KeyCode::Space) {
                if trans.translation.y <= -85. {
                    velocity.linear.y = player.jump_height;
                }
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
                        held_item.rotation = Quat::from_rotation_z(angle);
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
