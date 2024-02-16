mod camera;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_mod_picking::prelude::*;
use camera::PanOrbitCamera;

fn _render_origin(mut gizmos: Gizmos) {
    gizmos.line(Vec3::ZERO, Vec3::X, Color::RED);
    gizmos.line(Vec3::ZERO, Vec3::Y, Color::GREEN);
    gizmos.line(Vec3::ZERO, Vec3::Z, Color::BLUE);
}

pub fn main() {
    App::new()
        .insert_resource(ClearColor(Color::ANTIQUE_WHITE))
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 0.35,
        })
        .insert_resource(RapierConfiguration {
            timestep_mode: TimestepMode::Fixed { dt: 0.05, substeps: 20 },
            // // note that picking will not work if (query) pipeline is not active
            // physics_pipeline_active: false,
            // query_pipeline_active: false,
            //gravity: Vec3::ZERO,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultPickingPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        //.add_plugins(RapierDebugRenderPlugin::default())
        .add_systems(Startup, setup)
        //.add_systems(Update, render_origin)
        .add_systems(Update, (camera::update_camera_system, camera::accumulate_mouse_events_system))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // pan-orbit camera with relative directional light
    let translation = Vec3::new(0.5, 0.5, 0.5);
    let focus = Vec3::new(0.0, 0.1, 0.0);
    let transform = Transform::from_translation(translation)
        .looking_at(focus, Vec3::Y);
    commands
        .spawn(Camera3dBundle {
            transform,
            ..default()
        })
        .insert(RapierPickCamera::default())
        .insert(PanOrbitCamera {
            focus,
            radius: translation.length(),
            ..default()
        })
        .insert((ComputedVisibility::default(), Visibility::default()))
        .with_children(|commands| {
            commands.spawn(DirectionalLightBundle {
                directional_light: DirectionalLight {
                    shadows_enabled: false,
                    illuminance: 10000.0,
                    ..default()
                },
                transform: Transform::from_xyz(-2.5, 2.5, 2.5)
                    .looking_at(Vec3::ZERO, Vec3::Y),
                ..default()
            });
        });

    // floor
    commands
        .spawn(Collider::cuboid(1.0, 0.1, 1.0))
        .insert(SpatialBundle::from_transform(Transform::from_xyz(0.0, -0.1, 0.0)))
        .with_children(|commands| {
            commands.spawn(PbrBundle {
                mesh: meshes.add(shape::Plane::from_size(1.0).into()),
                material: materials.add(Color::rgb(0.9, 0.9, 0.9).into()),
                transform: Transform::from_xyz(0.0, 0.1, 0.0),
                ..default()
            });
        });

    // box
    #[derive(Component)]
    struct DragTarget {
        camera: Entity,
        offset: Option<Vec3>,
    }

    commands
        .spawn((Collider::cuboid(0.05, 0.05, 0.05), RigidBody::Dynamic))
        .insert(ColliderMassProperties::Mass(1.0))
        .insert(SpatialBundle::from_transform(Transform::from_xyz(0.0, 0.05, 0.0)))
        .with_children(|commands| {
            commands.spawn(PbrBundle {
                mesh: meshes.add(shape::Box::new(0.1, 0.1, 0.1).into()),
                material: materials.add(Color::rgb(0.9, 0.1, 0.1).into()),
                ..default()
            });
        })
        .insert(ExternalImpulse::default())
        .insert(PickableBundle::default())
        
        // DRAG START
        .insert(On::<Pointer<DragStart>>::run(|
            listener: Listener<Pointer<DragStart>>,
            target: Query<&GlobalTransform, With<ExternalImpulse>>,
            mut commands: Commands| {
            
            let target_transform = target.get_single().unwrap();
            let mut entity_commands = commands.entity(listener.target());
            
            entity_commands.insert(DragTarget {
                camera: listener.hit.camera,
                offset: listener.hit.position.map(|position| target_transform.affine()
                    .inverse()
                    .transform_point3(position)
                ),
            });
        }))

        // DRAG
        .insert(On::<Pointer<Drag>>::run(|
            _: Listener<Pointer<Drag>>,
            mut target: Query<(&DragTarget, &GlobalTransform, &mut ExternalImpulse)>,
            cameras: Query<(&Camera, &GlobalTransform)>,
            windows: Query<&Window>,
            mut gizmos: Gizmos,
            | {
            if let Ok((drag_target, transform, mut external_impulse)) = target.get_single_mut() {
                let (camera, camera_transform) = cameras.get(drag_target.camera).unwrap();
                let window = windows.get_single().unwrap();

                let ray = window.cursor_position()
                    .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor));
                
                if let Some(ray) = ray {
                    let intersection = ray.intersect_plane(Vec3::ZERO, Vec3::Y);
                    if let Some(distance) = intersection {
                        
                        // apply force at an offset if the backend gave us this information
                        let application_point = match drag_target.offset {
                            Some(offset) => transform.transform_point(offset),
                            None => transform.translation(),
                        };

                        gizmos.sphere(application_point, Quat::default(), 0.005, Color::BLUE);
                        
                        // apply a force that is proportional to the distance between the object and the cursor
                        let impulse = (ray.get_point(distance) - application_point) * 1.0;
                        external_impulse.impulse = impulse;
                        if let Some(offset) = drag_target.offset {
                            external_impulse.torque_impulse = offset.cross(impulse);
                        }
                    }        
                }
            }
        }))

        // DRAG END
        .insert(On::<Pointer<DragEnd>>::target_remove::<DragTarget>());
        
}







