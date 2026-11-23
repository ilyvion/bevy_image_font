//! Based on
//! <https://github.com/bevyengine/bevy/blob/main/examples/app/headless_renderer.rs>
//!
//! 1. Render from camera to gpu-image render target
//! 2. Copy from gpu image to buffer using `image_copy_driver` in the `RenderGraph` schedule
//! 3. Copy from buffer to channel using `receive_image_from_buffer` after `RenderSystems::Render`
//! 4. Save from channel to random named file using `scene::update` at `PostUpdate` in `MainWorld`
//! 5. Exit if `single_image` setting is set

use std::fs::{self, create_dir_all, remove_file};
use std::io::{Cursor, ErrorKind};
use std::iter;
use std::num::NonZero;
use std::path::Path;
use std::process::{self, Command};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use bevy::app::ScheduleRunnerPlugin;
use bevy::camera::RenderTarget;
use bevy::core_pipeline::schedule::camera_driver;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::ecs::system::ScheduleSystem;
use bevy::image::TextureFormatPixelInfo as _;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode, PollType,
    TexelCopyBufferInfo, TexelCopyBufferLayout, TextureFormat, TextureUsages,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderGraph, RenderQueue};
use bevy::render::settings::{RenderCreation, WgpuFeatures, WgpuSettings};
use bevy::render::texture::GpuImage;
use bevy::render::{Extract, Render, RenderApp, RenderPlugin, RenderSystems};
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use bevy_asset_loader::asset_collection::AssetCollectionApp as _;
use bevy_asset_loader::loading_state::config::ConfigureLoadingState as _;
use bevy_asset_loader::loading_state::{LoadingState, LoadingStateAppExt as _};
use bevy_image_font::ImageFontPlugin;
use crossbeam_channel::{Receiver, Sender};
use image::ImageFormat;
use oxipng::Options;

use crate::visual_acceptance_testing::{SCREENSHOT_HEIGHT, SCREENSHOT_WIDTH, TestAssets};

// To communicate between the main world and the render world we need a channel.
// Since the main world and render world run in parallel, there will always be a
// frame of latency between the data sent from the render world and the data
// received in the main world
//
// frame n => render world sends data through the channel at the end of the
// frame frame n + 1 => main world receives the data
//
// Receiver and Sender are kept in resources because there is single camera and
// single target That's why there is single images role, if you want to
// differentiate images from different cameras, you should keep Receiver in
// ImageCopier and Sender in ImageToSave or send some id with data

/// This will receive asynchronously any data sent from the render world
#[derive(Resource, Deref)]
struct MainWorldReceiver(Receiver<Vec<u8>>);

/// This will send asynchronously any data to the main world
#[derive(Resource, Deref)]
struct RenderWorldSender(Sender<Vec<u8>>);

pub(crate) fn prepare_app<M>(
    category: impl Into<String>,
    image_name: impl Into<String>,
    setup_system: impl IntoScheduleConfigs<ScheduleSystem, M>,
) {
    let mut app = App::new();
    let category = category.into();
    let bindless = category != "rendered";

    setup_scene_controller(&mut app, category, image_name.into());
    setup_plugins(&mut app, bindless);
    setup_asset_loading(&mut app);
    setup_rendering(&mut app);

    app.add_systems(Startup, (setup, setup_system)).run();
}

fn setup_scene_controller(app: &mut App, category: String, image_name: String) {
    app.insert_resource(SceneController::new(
        SCREENSHOT_WIDTH,
        SCREENSHOT_HEIGHT,
        category,
        image_name,
    ))
    .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.2)));
}

/// Sets up the plugins, with bindless materials only enabled if `bindless` is
/// set.
///
/// Bindless materials make wgpu revalidate every texture in a shared bind group
/// on each draw. This makes scenes with thousands of distinct sprite images,
/// like the pre-rendered text tests, take seconds per frame, while scenes that
/// share a few atlas images render faster with them.
fn setup_plugins(app: &mut App, bindless: bool) {
    let disabled_features = (!bindless).then_some(WgpuFeatures::TEXTURE_BINDING_ARRAY);

    app.add_plugins(
        DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(Box::new(WgpuSettings {
                    disabled_features,
                    ..default()
                })),
                // With asynchronous compilation, frames captured before the pipelines are
                // ready are blank. The capture doesn't wait any frames for them.
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .disable::<WinitPlugin>()
            .disable::<LogPlugin>(),
    )
    .add_plugins(ImageFontPlugin);
}

fn setup_asset_loading(app: &mut App) {
    app.init_collection::<TestAssets>()
        .init_state::<AssetLoadState>()
        .add_loading_state(
            LoadingState::new(AssetLoadState::Loading)
                .load_collection::<TestAssets>()
                .continue_to_state(AssetLoadState::Done),
        )
        .add_systems(OnEnter(AssetLoadState::Done), transition_to_assets_loaded);
}

const FRAME_RATE: f64 = 1.0 / 60.0;

fn setup_rendering(app: &mut App) {
    app.add_plugins(ImageCopyPlugin)
        .add_plugins(CaptureFramePlugin)
        .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            FRAME_RATE,
        )))
        .init_resource::<SceneController>();
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, States)]
enum AssetLoadState {
    #[default]
    Loading,
    Done,
}

fn transition_to_assets_loaded(mut scene_controller: ResMut<SceneController>) {
    scene_controller.state = SceneState::AssetsLoaded;
}

/// Capture image settings and state
#[derive(Debug, Default, Resource)]
struct SceneController {
    state: SceneState,
    name: String,
    width: u32,
    height: u32,
    test_category: String,
    test_name: String,
}

impl SceneController {
    fn new(width: u32, height: u32, test_category: String, test_name: String) -> SceneController {
        SceneController {
            state: SceneState::BuildScene,
            name: String::new(),
            width,
            height,
            test_category,
            test_name,
        }
    }
}

/// Capture image state
#[derive(Debug, Default)]
enum SceneState {
    #[default]
    // State before any rendering takes place
    BuildScene,
    // Assets are loaded; frames captured before this point are discarded
    AssetsLoaded,
    // Render the scene and save to image
    Render,
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut scene_controller: ResMut<SceneController>,
    render_device: Res<RenderDevice>,
) {
    let render_target = setup_render_target(
        &mut commands,
        &mut images,
        &render_device,
        &mut scene_controller,
        "main_scene".into(),
    );

    commands.spawn((Camera2d, render_target, Tonemapping::None));
}

/// Plugin for Render world part of work
#[derive(Debug)]
pub(crate) struct ImageCopyPlugin;
impl Plugin for ImageCopyPlugin {
    fn build(&self, app: &mut App) {
        let (sender, receiver) = crossbeam_channel::unbounded();

        let render_app = app
            .insert_resource(MainWorldReceiver(receiver))
            .sub_app_mut(RenderApp);

        render_app
            .insert_resource(RenderWorldSender(sender))
            // Make ImageCopiers accessible in RenderWorld system and plugin
            .add_systems(ExtractSchedule, image_copy_extract)
            // Receives image data from buffer to channel
            // so we need to run it after the render graph is done
            .add_systems(
                Render,
                receive_image_from_buffer.after(RenderSystems::Render),
            )
            .add_systems(RenderGraph, image_copy_driver.after(camera_driver));
    }
}

/// Setups render target and cpu image for saving, changes scene state into
/// render mode
fn setup_render_target(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    render_device: &Res<RenderDevice>,
    scene_controller: &mut ResMut<SceneController>,
    scene_name: String,
) -> RenderTarget {
    let size = Extent3d {
        width: scene_controller.width,
        height: scene_controller.height,
        ..Default::default()
    };

    let render_target_image_handle = create_render_target_image(images, size);
    let cpu_image_handle = create_cpu_image(images, size);

    commands.spawn(ImageCopier::new(
        render_target_image_handle.clone(),
        size,
        render_device,
    ));

    commands.spawn(ImageToSave(cpu_image_handle));

    scene_controller.name = scene_name;
    RenderTarget::Image(render_target_image_handle.into())
}

fn create_render_target_image(images: &mut ResMut<Assets<Image>>, size: Extent3d) -> Handle<Image> {
    let mut render_target_image =
        Image::new_target_texture(size.width, size.height, TextureFormat::Rgba8UnormSrgb, None);
    render_target_image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    images.add(render_target_image)
}

fn create_cpu_image(images: &mut ResMut<Assets<Image>>, size: Extent3d) -> Handle<Image> {
    let cpu_image =
        Image::new_target_texture(size.width, size.height, TextureFormat::Rgba8UnormSrgb, None);
    images.add(cpu_image)
}

/// Setups image saver
#[derive(Debug)]
pub(crate) struct CaptureFramePlugin;
impl Plugin for CaptureFramePlugin {
    fn build(&self, app: &mut App) {
        info!("Adding CaptureFramePlugin");
        app.add_systems(PostUpdate, update);
    }
}

/// `ImageCopier` aggregator in `RenderWorld`
#[derive(Clone, Default, Resource, Deref, DerefMut)]
struct ImageCopiers(pub Vec<ImageCopier>);

/// Used by `image_copy_driver` for copying from render target to buffer
#[derive(Clone, Component)]
struct ImageCopier {
    buffer: Buffer,
    enabled: Arc<AtomicBool>,
    src_image: Handle<Image>,
}

impl ImageCopier {
    fn new(src_image: Handle<Image>, size: Extent3d, render_device: &RenderDevice) -> ImageCopier {
        let padded_bytes_per_row =
            RenderDevice::align_copy_bytes_per_row((size.width) as usize) * 4;

        let cpu_buffer = render_device.create_buffer(&BufferDescriptor {
            label: None,
            size: padded_bytes_per_row as u64 * u64::from(size.height),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        ImageCopier {
            buffer: cpu_buffer,
            src_image,
            enabled: Arc::new(AtomicBool::new(true)),
        }
    }

    fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

/// Extracting `ImageCopier`s into render world, because `image_copy_driver`
/// accesses them
fn image_copy_extract(mut commands: Commands, image_copiers: Extract<Query<&ImageCopier>>) {
    commands.insert_resource(ImageCopiers(
        image_copiers.iter().cloned().collect::<Vec<ImageCopier>>(),
    ));
}

/// Copies image content from render target to buffer
#[expect(
    clippy::unwrap_used,
    reason = "we want to panic if we're wrong about any of these"
)]
fn image_copy_driver(
    render_context: RenderContext,
    image_copiers: Res<ImageCopiers>,
    render_queue: Res<RenderQueue>,
    gpu_images: Res<RenderAssets<GpuImage>>,
) {
    for image_copier in image_copiers.iter() {
        if !image_copier.enabled() {
            continue;
        }

        let src_image = gpu_images.get(&image_copier.src_image).unwrap();

        let mut encoder = render_context
            .render_device()
            .create_command_encoder(&CommandEncoderDescriptor::default());

        let format = src_image.texture_descriptor.format;
        let size = src_image.texture_descriptor.size;
        let block_dimensions = format.block_dimensions();
        let block_size = format.block_copy_size(None).unwrap();

        // Calculating correct size of image row because
        // copy_texture_to_buffer can copy image only by rows aligned
        // wgpu::COPY_BYTES_PER_ROW_ALIGNMENT That's why image in buffer can
        // be little bit wider This should be taken into account at copy
        // from buffer stage
        let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
            (size.width as usize / block_dimensions.0 as usize) * block_size as usize,
        );

        encoder.copy_texture_to_buffer(
            src_image.texture.as_image_copy(),
            TexelCopyBufferInfo {
                buffer: &image_copier.buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    #[expect(
                        clippy::cast_possible_truncation,
                        reason = "while usize can hold more data than u32, we're working on a \
                        number here that should be substantially smaller than even u32's \
                        capacity"
                    )]
                    bytes_per_row: Some(
                        NonZero::<u32>::new(padded_bytes_per_row as u32)
                            .map(Into::into)
                            .unwrap(),
                    ),
                    rows_per_image: None,
                },
            },
            size,
        );

        render_queue.submit(iter::once(encoder.finish()));
    }
}

/// runs in render world after Render stage to send image from buffer via
/// channel (receiver is in main world)
#[expect(
    clippy::expect_used,
    reason = "we want to panic if any of these go wrong"
)]
fn receive_image_from_buffer(
    image_copiers: Res<ImageCopiers>,
    render_device: Res<RenderDevice>,
    sender: Res<RenderWorldSender>,
) {
    for image_copier in &image_copiers.0 {
        if !image_copier.enabled() {
            continue;
        }

        // Finally time to get our data back from the gpu.
        // First we get a buffer slice which represents a chunk of the buffer (which we
        // can't access yet).
        // We want the whole thing so use unbounded range.
        let buffer_slice = image_copier.buffer.slice(..);

        // Now things get complicated. WebGPU, for safety reasons, only allows either
        // the GPU or CPU to access a buffer's contents at a time. We need to
        // "map" the buffer which means flipping ownership of the buffer over to
        // the CPU and making access legal. We do this
        // with `BufferSlice::map_async`.
        //
        // The problem is that map_async is not an async function so we can't await it.
        // What we need to do instead is pass in a closure that will be executed
        // when the slice is either mapped or the mapping has failed.
        //
        // The problem with this is that we don't have a reliable way to wait in the
        // main code for the buffer to be mapped and even worse, calling
        // get_mapped_range or get_mapped_range_mut prematurely will cause a
        // panic, not return an error.
        //
        // Using channels solves this as awaiting the receiving of a message from
        // the passed closure will force the outside code to wait. It also doesn't hurt
        // if the closure finishes before the outside code catches up as the message is
        // buffered and receiving will just pick that up.
        //
        // It may also be worth noting that although on native, the usage of
        // asynchronous channels is wholly unnecessary, for the sake of
        // portability to Wasm we'll use async channels that work on both native
        // and Wasm.

        let (buffer_sender, buffer_receiver) = crossbeam_channel::bounded(1);

        // Maps the buffer so it can be read on the cpu
        buffer_slice.map_async(MapMode::Read, move |result| match result {
            // This will execute once the gpu is ready, so after the call to poll()
            Ok(result) => buffer_sender
                .send(result)
                .expect("Failed to send map update"),
            Err(err) => panic!("Failed to map buffer {err}"),
        });

        // In order for the mapping to be completed, one of three things must happen.
        // One of those can be calling `Device::poll`. This isn't necessary on the web
        // as devices are polled automatically but natively, we need to make
        // sure this happens manually. `Maintain::Wait` will cause the thread to
        // wait on native but not on WebGpu.

        // This blocks until the gpu is done executing everything
        render_device
            .poll(PollType::wait_indefinitely())
            .expect("Failed to poll device for map async");

        // This blocks until the buffer is mapped
        buffer_receiver
            .recv()
            .expect("Failed to receive the map_async message");

        // This could fail on app exit, if Main world clears resources (including
        // receiver) while Render world still renders
        let mapped_range = buffer_slice
            .get_mapped_range()
            .expect("Failed to get the mapped buffer range");
        drop(sender.send(mapped_range.to_vec()));
        drop(mapped_range);

        // We need to make sure all `BufferView`'s are dropped before we do what we're
        // about to do.
        // Unmap so that we can copy to the staging buffer in the next iteration.
        image_copier.buffer.unmap();
    }
}

/// CPU-side image for saving
#[derive(Component, Deref, DerefMut)]
struct ImageToSave(Handle<Image>);

fn update(
    images_to_save: Query<&ImageToSave>,
    receiver: Res<MainWorldReceiver>,
    images: ResMut<Assets<Image>>,
    mut scene_controller: ResMut<SceneController>,
    app_exit_writer: MessageWriter<AppExit>,
) {
    match scene_controller.state {
        SceneState::BuildScene => {}
        SceneState::AssetsLoaded => {
            clear_receiver(&receiver);
            scene_controller.state = SceneState::Render;
        }
        SceneState::Render => render_scene(
            images_to_save,
            receiver,
            images,
            scene_controller,
            app_exit_writer,
        ),
    }
}

fn clear_receiver(receiver: &Res<MainWorldReceiver>) {
    for _ in receiver.try_iter() {
        // Do nothing, just consume messages
    }
}

#[expect(
    clippy::unwrap_used,
    reason = "we want to panic if any of these go wrong"
)]
fn render_scene(
    images_to_save: Query<&ImageToSave>,
    receiver: Res<MainWorldReceiver>,
    mut images: ResMut<Assets<Image>>,
    scene_controller: ResMut<SceneController>,
    app_exit_writer: MessageWriter<AppExit>,
) {
    let image_data = fetch_latest_image_data(&receiver);
    if image_data.is_empty() {
        return;
    }

    let image = images_to_save.single().unwrap();
    let mut img_bytes = images.get_mut(image.id()).unwrap();
    let img = prepare_image_buffer(image_data, &mut img_bytes);

    let image_paths = ImagePaths::new(
        &scene_controller.test_category,
        &scene_controller.test_name,
        process::id(),
    );
    image_paths.create_dirs();

    // write_optimized_png(img, &image_paths.work_new);
    if let Err(error) = img.save(&image_paths.work_new) {
        panic!("Failed to save image: {error}");
    }

    // If this is a new test, we're done, panic to indicate what to do next.
    if !image_paths.accepted.exists() {
        image_paths.publish_failure_images();
        panic!(
            "Visual acceptance test `{}` has no accepted image to compare against. \
             If this is the first time running this test, inspect {} and rename it to {} if \
             it passes initial inspection.",
            scene_controller.test_name,
            image_paths.new.display(),
            image_paths.accepted.display(),
        );
    }

    compare_images(&image_paths, app_exit_writer);
}

#[expect(
    clippy::expect_used,
    reason = "we want to panic if any of these go wrong"
)]
fn compare_images(paths: &ImagePaths, mut app_exit_writer: MessageWriter<AppExit>) {
    // Invoke ImageMagick Compare to check if the output has changed
    let output = Command::new("magick")
        .args([
            "compare".as_ref(),
            "-verbose".as_ref(),
            "-metric".as_ref(),
            "psnr".as_ref(),
            paths.accepted.as_os_str(),
            paths.work_new.as_os_str(),
            paths.work_diff.as_os_str(),
        ])
        .output()
        .expect("Failed to run ImageMagick Compare");

    if !output.status.success() {
        paths.publish_failure_images();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        panic!(
            "Visual mismatch detected.\n\
             See {} for the diff image and {} for the newly generated output.\n\n\
             ImageMagick standard output:\n{}\n\n\
             ImageMagick standard error:\n{}",
            paths.diff.display(),
            paths.new.display(),
            stdout,
            stderr
        );
    }

    paths.clean_up_after_pass();

    // Once we're done producing and comparing our images and we got this far, it's
    // time to exit with success to indicate nothing needs doing.
    app_exit_writer.write(AppExit::Success);
}

fn fetch_latest_image_data(receiver: &MainWorldReceiver) -> Vec<u8> {
    // We don't want to block the main world on this, so we use try_iter which
    // receives without blocking. Image generation could be faster than saving to
    // fs, that's why use only the last of them.
    receiver.try_iter().last().unwrap_or_default()
}

#[expect(
    clippy::unwrap_used,
    reason = "we want to panic if any of these go wrong"
)]
fn prepare_image_buffer(
    image_data: Vec<u8>,
    img_bytes: &mut Image,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    // We need to ensure that this works regardless of the image dimensions
    // If the image became wider when copying from the texture to the buffer,
    // then the data is reduced to its original size when copying from the buffer to
    // the image.
    let row_bytes =
        img_bytes.width() as usize * img_bytes.texture_descriptor.format.pixel_size().unwrap();
    let aligned_row_bytes = RenderDevice::align_copy_bytes_per_row(row_bytes);

    // If row_bytes == aligned_row_bytes, we can copy directly. Otherwise, we must
    // adjust alignment.
    if row_bytes == aligned_row_bytes {
        img_bytes.data.as_mut().unwrap().clone_from(&image_data);
    } else {
        // Extract only the meaningful part of each row, ignoring padding
        img_bytes.data = Some(
            image_data
                .chunks(aligned_row_bytes)
                .take(img_bytes.height() as usize)
                .flat_map(|row| &row[..row_bytes.min(row.len())])
                .copied()
                .collect(),
        );
    }

    // Create RGBA Image Buffer
    img_bytes.clone().try_into_dynamic().map_or_else(
        |error| panic!("Failed to create image buffer: {error:?}"),
        |img| img.to_rgba8(),
    )
}

/// The files a visual test reads and writes.
///
/// The test saves and compares its output in `work_new` and `work_diff`, which
/// are private to the test process, so test runs that overlap on the same
/// checkout don't touch each other's files. Only a failing test copies them to
/// `new` and `diff` next to the accepted image.
#[derive(Debug)]
struct ImagePaths {
    accepted: PathBuf,
    new: PathBuf,
    diff: PathBuf,
    work_new: PathBuf,
    work_diff: PathBuf,
}

impl ImagePaths {
    fn new(category: &str, test_name: &str, process_id: u32) -> ImagePaths {
        let images_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("images")
            .join(category);
        let work_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("visual")
            .join(process_id.to_string())
            .join(category);

        ImagePaths {
            accepted: images_dir.join(format!("{test_name}.accepted.png")),
            new: images_dir.join(format!("{test_name}.new.png")),
            diff: images_dir.join(format!("{test_name}.diff.png")),
            work_new: work_dir.join(format!("{test_name}.new.png")),
            work_diff: work_dir.join(format!("{test_name}.diff.png")),
        }
    }

    fn create_dirs(&self) {
        for path in [&self.new, &self.work_new] {
            if let Some(dir) = path.parent() {
                create_dir_all(dir).unwrap_or_else(|error| {
                    panic!(
                        "Failed to create image directory {}: {error}",
                        dir.display()
                    )
                });
            }
        }
    }

    /// Copies the work images that exist to `new` and `diff`, then removes the
    /// work images.
    fn publish_failure_images(&self) {
        for (work, published) in [(&self.work_new, &self.new), (&self.work_diff, &self.diff)] {
            if work.exists() {
                copy_replacing(work, published);
            }
            remove_if_exists(work);
        }
        self.remove_empty_work_dirs();
    }

    /// Removes the work images and any `new` and `diff` images left by an
    /// earlier failed run.
    fn clean_up_after_pass(&self) {
        for path in [&self.work_new, &self.work_diff, &self.new, &self.diff] {
            remove_if_exists(path);
        }
        self.remove_empty_work_dirs();
    }

    /// Removes the work image directory and its parent, leaving any that still
    /// hold another test's files.
    fn remove_empty_work_dirs(&self) {
        for dir in self.work_new.ancestors().skip(1).take(2) {
            if fs::remove_dir(dir).is_err() {
                break;
            }
        }
    }
}

/// Copies `from` to `to` through a temporary file in `to`'s directory, so a
/// concurrent copy to the same `to` can't interleave with this one.
fn copy_replacing(from: &Path, to: &Path) {
    let file_name = to
        .file_name()
        .map_or_else(Default::default, |name| name.to_string_lossy().into_owned());
    let temporary = to.with_file_name(format!(".{file_name}.{}.tmp", process::id()));

    if let Err(error) = fs::copy(from, &temporary) {
        panic!(
            "Failed to copy {} to {}: {error}",
            from.display(),
            temporary.display()
        );
    }
    if let Err(error) = fs::rename(&temporary, to) {
        panic!(
            "Failed to rename {} to {}: {error}",
            temporary.display(),
            to.display()
        );
    }
}

fn remove_if_exists(path: &Path) {
    match remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => panic!("Failed to remove {}: {error}", path.display()),
    }
}

#[expect(
    unused,
    reason = "This function is slow and should only be used to generate the final \
acceptance image before committing. As an alternative, you can also run the `oxipng` program \
directly on the non-optimized files."
)]
fn write_optimized_png(img: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>, new_image_path: &Path) {
    let mut new_image = Vec::new();
    if let Err(error) = img.write_to(&mut Cursor::new(&mut new_image), ImageFormat::Png) {
        panic!("Failed to write image as PNG: {error}");
    }

    let new_image = match oxipng::optimize_from_memory(&new_image, &Options::max_compression()) {
        Ok(new_image) => new_image,
        Err(error) => panic!("Failed to optimize image: {error}"),
    };

    if let Err(error) = fs::write(new_image_path, new_image) {
        panic!("Failed to save image: {error}");
    }
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    reason = "we want to panic if any of these go wrong"
)]
mod tests {
    use super::*;

    fn images_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("images")
    }

    #[test]
    fn test_processes_save_and_compare_in_separate_directories() {
        let first = ImagePaths::new("rendered", "sizes", 1);
        let second = ImagePaths::new("rendered", "sizes", 2);

        let first_work = [&first.work_new, &first.work_diff];
        let second_work = [&second.work_new, &second.work_diff];

        for path in first_work.iter().chain(&second_work) {
            assert!(
                !path.starts_with(images_dir()),
                "{} is shared by every test run on the checkout",
                path.display()
            );
        }
        for path in first_work {
            assert!(
                !second_work.contains(&path),
                "{} is used by two test processes",
                path.display()
            );
        }
    }

    #[test]
    fn failure_images_are_published_next_to_the_accepted_image() {
        let paths = ImagePaths::new("rendered", "sizes", 1);

        assert_eq!(paths.new.parent(), paths.accepted.parent());
        assert_eq!(paths.diff.parent(), paths.accepted.parent());
        assert!(paths.accepted.starts_with(images_dir()));
    }

    /// `ImagePaths` in a scratch directory that is removed on drop.
    #[derive(Debug, Deref)]
    struct ScratchPaths {
        root: PathBuf,
        #[deref]
        paths: ImagePaths,
    }

    impl Drop for ScratchPaths {
        fn drop(&mut self) {
            drop(fs::remove_dir_all(&self.root));
        }
    }

    fn scratch_paths(test: &str) -> ScratchPaths {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("visual-setup-tests")
            .join(format!("{test}-{}", process::id()));
        drop(fs::remove_dir_all(&root));

        let paths = ImagePaths {
            accepted: root.join("published").join("a.accepted.png"),
            new: root.join("published").join("a.new.png"),
            diff: root.join("published").join("a.diff.png"),
            work_new: root.join("work").join("a.new.png"),
            work_diff: root.join("work").join("a.diff.png"),
        };
        paths.create_dirs();
        ScratchPaths { root, paths }
    }

    #[test]
    fn publishing_failure_images_copies_them_and_removes_the_work_images() {
        let paths = scratch_paths("publish");
        fs::write(&paths.work_new, "new").unwrap();
        fs::write(&paths.work_diff, "diff").unwrap();

        paths.publish_failure_images();

        assert_eq!(fs::read_to_string(&paths.new).unwrap(), "new");
        assert_eq!(fs::read_to_string(&paths.diff).unwrap(), "diff");
        assert!(!paths.work_new.exists());
        assert!(!paths.work_diff.exists());
    }

    #[test]
    fn publishing_failure_images_without_a_diff_publishes_only_the_new_image() {
        let paths = scratch_paths("publish-no-diff");
        fs::write(&paths.work_new, "new").unwrap();

        paths.publish_failure_images();

        assert_eq!(fs::read_to_string(&paths.new).unwrap(), "new");
        assert!(!paths.diff.exists());
        assert!(!paths.work_new.exists());
    }

    #[test]
    fn passing_removes_work_images_and_leftover_failure_images() {
        let paths = scratch_paths("pass");
        for path in [&paths.work_new, &paths.work_diff, &paths.new, &paths.diff] {
            fs::write(path, "x").unwrap();
        }

        paths.clean_up_after_pass();

        for path in [&paths.work_new, &paths.work_diff, &paths.new, &paths.diff] {
            assert!(!path.exists(), "{} was not removed", path.display());
        }
    }

    #[test]
    fn finishing_removes_the_emptied_work_directory() {
        let passed = scratch_paths("pass-work-dir");
        fs::write(&passed.work_new, "x").unwrap();
        passed.clean_up_after_pass();
        assert!(!passed.work_new.parent().unwrap().exists());

        let failed = scratch_paths("fail-work-dir");
        fs::write(&failed.work_new, "x").unwrap();
        failed.publish_failure_images();
        assert!(!failed.work_new.parent().unwrap().exists());
    }

    #[test]
    fn passing_tolerates_files_already_removed() {
        let paths = scratch_paths("pass-missing");

        paths.clean_up_after_pass();
    }
}
