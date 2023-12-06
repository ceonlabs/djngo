use bevy::prelude::*;
use arrow::ffi_stream::{ArrowArrayStreamReader, FFI_ArrowArrayStream};
use gdal::cpl::CslStringList;
use gdal::vector::*;
use gdal::Dataset;
use std::path::Path;



fn run() -> gdal::errors::Result<()> {
    // Open a dataset and access a layer
    let dataset_a = Dataset::open(Path::new("fixtures/roads.geojson"))?;
    let mut layer_a = dataset_a.layer(0)?;

    // Instantiate an `ArrowArrayStream` for OGR to write into
    let mut output_stream = Box::new(FFI_ArrowArrayStream::empty());

    // Access the unboxed pointer
    let output_stream_ptr = &mut *output_stream as *mut FFI_ArrowArrayStream;

    // gdal includes its own copy of the ArrowArrayStream struct definition. These are guaranteed
    // to be the same across implementations, but we need to manually cast between the two for Rust
    // to allow it.
    let gdal_pointer: *mut gdal::ArrowArrayStream = output_stream_ptr.cast();

    let mut options = CslStringList::new();
    options.set_name_value("INCLUDE_FID", "NO")?;

    // Read the layer's data into our provisioned pointer
    unsafe { layer_a.read_arrow_stream(gdal_pointer, &options).unwrap() }

    // The rest of this example is arrow2-specific.

    // `arrow2` has a helper class `ArrowArrayStreamReader` to assist with iterating over the raw
    // batches
    let arrow_stream_reader =
        unsafe { ArrowArrayStreamReader::from_raw(output_stream_ptr).unwrap() };

    // Iterate over the stream until it's finished
    for maybe_record_batch in arrow_stream_reader {
        // Access the contained array
        let _record_batch = maybe_record_batch.unwrap();

        // // The top-level array is a single logical "struct" array which includes all columns of the
        // // dataset inside it.
        // assert!(
        //     matches!(top_level_array.data_type(), DataType::Struct(..)),
        //     "Top-level arrays from OGR are expected to be of struct type"
        // );

        // // Downcast from the Box<dyn Array> to a concrete StructArray
        // let struct_array = top_level_array
        //     .as_any()
        //     .downcast_ref::<StructArray>()
        //     .unwrap();

        // // Access the underlying column metadata and data
        // // Clones are cheap because they do not copy the underlying data
        // let (fields, columns, _validity) = struct_array.clone().into_data();

        // // Find the index of the geometry column
        // let geom_column_index = fields
        //     .iter()
        //     .position(|field| field.name == "wkb_geometry")
        //     .unwrap();

        // // Pick that column and downcast to a BinaryArray
        // let geom_column = &columns[geom_column_index];
        // let binary_array = geom_column
        //     .as_any()
        //     .downcast_ref::<BinaryArray<i32>>()
        //     .unwrap();

        // let wkb_array = WKBArray::new(binary_array.clone());
        // let line_string_array: LineStringArray<i32> = wkb_array.try_into().unwrap();

        // let geodesic_length = line_string_array.geodesic_length();

        // println!("Number of geometries: {}", line_string_array.len());
        // println!("Geodesic Length: {:?}", geodesic_length);
    }

    Ok(())
}



fn main() {
    // Load the polygons file for the virtual world
    run().unwrap()
    // Start the music
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_positions)
        .add_systems(Update, update_listener)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Space between the two ears
    let gap = 4.0;

    // sound emitter
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.2,
                ..default()
            })),
            material: materials.add(Color::BLUE.into()),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        Emitter::default(),
        AudioBundle {
            source: asset_server.load("sounds/Windless Slopes.ogg"),
            settings: PlaybackSettings::LOOP.with_spatial(true),
        },
    ));

    let listener = SpatialListener::new(gap);
    commands
        .spawn((SpatialBundle::default(), listener.clone()))
        .with_children(|parent| {
            // left ear indicator
            parent.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.2 })),
                material: materials.add(Color::RED.into()),
                transform: Transform::from_translation(listener.left_ear_offset),
                ..default()
            });

            // right ear indicator
            parent.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.2 })),
                material: materials.add(Color::GREEN.into()),
                transform: Transform::from_translation(listener.right_ear_offset),
                ..default()
            });
        });

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // example instructions
    commands.spawn(
        TextBundle::from_section(
            "Up/Down/Left/Right: Move Listener\nSpace: Toggle Emitter Movement",
            TextStyle {
                font_size: 20.0,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Component, Default)]
struct Emitter {
    stopped: bool,
}

fn update_positions(
    time: Res<Time>,
    mut emitters: Query<(&mut Transform, &mut Emitter), With<Emitter>>,
    keyboard: Res<Input<KeyCode>>,
) {
    for (mut emitter_transform, mut emitter) in emitters.iter_mut() {
        if keyboard.just_pressed(KeyCode::Space) {
            emitter.stopped = !emitter.stopped;
        }

        if !emitter.stopped {
            emitter_transform.translation.x = time.elapsed_seconds().sin() * 3.0;
            emitter_transform.translation.z = time.elapsed_seconds().cos() * 3.0;
        }
    }
}

fn update_listener(
    keyboard: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut listeners: Query<&mut Transform, With<SpatialListener>>,
) {
    let mut transform = listeners.single_mut();

    let speed = 2.;

    if keyboard.pressed(KeyCode::Right) {
        transform.translation.x += speed * time.delta_seconds();
    }
    if keyboard.pressed(KeyCode::Left) {
        transform.translation.x -= speed * time.delta_seconds();
    }
    if keyboard.pressed(KeyCode::Down) {
        transform.translation.z += speed * time.delta_seconds();
    }
    if keyboard.pressed(KeyCode::Up) {
        transform.translation.z -= speed * time.delta_seconds();
    }
}
