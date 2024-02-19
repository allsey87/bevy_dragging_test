mod camera;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_mod_picking::prelude::*;
use camera::PanOrbitCamera;

fn render_origin(mut gizmos: Gizmos) {
    gizmos.line(Vec3::ZERO, Vec3::X, Color::RED);
    gizmos.line(Vec3::ZERO, Vec3::Y, Color::GREEN);
    gizmos.line(Vec3::ZERO, Vec3::Z, Color::BLUE);
}

#[derive(Component)]
struct DragTarget {
    // the camera on which this drag is occuring
    camera: Entity,

    // allows calculating the drag target from the mouse
    origin: Vec3,

    // the offset from the center of mass where the drag started
    offset: Vec3,

    // distance of the drag (as last reported by events<pointer<drag>>)
    distance: Vec2,
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
        .add_systems(Update, render_origin)
        .add_systems(Update, (camera::update_camera_system, camera::accumulate_mouse_events_system))
        .add_systems(Update, drag_system)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // pan-orbit camera with relative directional light
    let translation = Vec3::new(0.0, 0.5, 0.0);
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
                material: materials.add(Color::rgba(0.9, 0.9, 0.9, 0.5).into()),
                transform: Transform::from_xyz(0.0, 0.1, 0.0),
                ..default()
            });
        });

    // box
    commands
        .spawn((Collider::cuboid(0.05, 0.05, 0.05), RigidBody::Dynamic))
        .insert(ColliderMassProperties::Mass(1.0))
        .insert(SpatialBundle::from_transform(Transform::from_xyz(0.0, 0.05, 0.0)))
        .with_children(|commands| {
            commands.spawn(PbrBundle {
                mesh: meshes.add(shape::Box::new(0.1, 0.1, 0.1).into()),
                material: materials.add(Color::RED.into()),
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
            if listener.button == PointerButton::Primary {
                let target_transform = target.get_single().unwrap();
                let position = listener.hit.position
                    .expect("backend does not support `position`");
                commands.entity(listener.target()).insert(DragTarget {
                    camera: listener.hit.camera,
                    origin: position,
                    offset: target_transform.affine().inverse().transform_point3(position),
                    distance: Default::default()
                });
            }
        }))
        // DRAG END
        .insert(On::<Pointer<DragEnd>>::target_remove::<DragTarget>());
}

fn drag_system(
    mut drag_events: EventReader<Pointer<Drag>>,
    mut target: Query<(&mut DragTarget, &GlobalTransform, &mut ExternalImpulse)>,
    camera_transforms: Query<&GlobalTransform, With<Camera>>,
) {
    if let Ok((mut target, target_transform, mut target_force)) = target.get_single_mut() {
        /* update the cached target distance */
        if let Some(last_drag_event) = drag_events.iter().last() {
            target.distance = last_drag_event.distance;
        }

        /* convert drag target distance  */
        let camera_transform = camera_transforms
            .get(target.camera)
            .unwrap();
        let mut drag_target_offset = camera_transform.translation() +
            target.distance.x * camera_transform.right() -
            target.distance.y * camera_transform.up();
        drag_target_offset.y = 0.0;

        // TODO: improve zoom factor for lower camera altitudes
        let zoom_factor = (camera_transform.translation() - target.origin).length() * 0.0011;
        let drag_target = target.origin + (drag_target_offset * zoom_factor);
        let drag_point = target_transform.transform_point(target.offset);

        // TODO: make gain a factor of object weight
        const GAIN: f32 = 1.5;
        // TODO: use PID control?
        let drag_impulse = (drag_target - drag_point)
            .clamp(Vec3::NEG_ONE, Vec3::ONE) * GAIN;
        target_force.impulse = drag_impulse;

        let mut drag_com_offset = drag_point - target_transform.translation();
        drag_com_offset.y = 0.0;

        let orthogonal_vector = (drag_com_offset) - (drag_com_offset).project_onto(drag_impulse);
        target_force.torque_impulse = orthogonal_vector.cross(drag_impulse);
    }
}






