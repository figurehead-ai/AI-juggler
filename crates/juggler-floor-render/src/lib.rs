//! The Floor's renderer (ADR-0026, AJ-E3507): draws a
//! [`juggler_floor::scene::Scene`] into a window's surface or offscreen.
//!
//! - The surface comes from wgpu's safe `create_surface` with the window
//!   as an owned handle (made on the main thread by the app); this crate
//!   has no unsafe code.
//! - Each frame builds the scene's moving part and the overlay on the CPU
//!   (pure, in `juggler-floor`) and draws four passes (`gpu`).
//! - Picking reads one pixel of the last frame's id buffer; capture draws
//!   offscreen for the gallery's golden frames.
//! - A refused frame (the window covered, or its surface out of date) is
//!   reported, never dropped: the loop retries it.

// Pixel and metre geometry on small numbers: the casts between them lose
// nothing that matters; pixels have their own short names (x, y, w, h).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

mod gpu;
mod shaders;

/// The GPU library, for the window's instance and surface.
pub use wgpu;

use juggler_floor::camera::View;
use juggler_floor::driver::Chip;
use juggler_floor::models::Item;
use juggler_floor::overlay;
use juggler_floor::paint::Img;
use juggler_floor::scene::{Live, Scene};

pub use crate::gpu::pixel_size;
use crate::gpu::{Frame, Gpu};

/// Why the renderer couldn't start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// No GPU adapter, not even a software one.
    NoAdapter(String),
    /// The adapter gave no device.
    NoDevice(String),
    /// The window's surface couldn't be made or read.
    Surface(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAdapter(why) => write!(f, "no graphics adapter: {why}"),
            Self::NoDevice(why) => write!(f, "the graphics adapter gave no device: {why}"),
            Self::Surface(why) => write!(f, "the window can't be drawn into: {why}"),
        }
    }
}

impl std::error::Error for Error {}

/// What else a frame shows besides the scene.
#[derive(Clone, Copy, Debug)]
pub struct Extras<'a> {
    /// How to move the view, in this system's words.
    pub hint: &'a str,
    /// The overload alert's words, while it's on.
    pub alert: Option<&'a str>,
    /// The pointer, device pixels.
    pub mouse: Option<(f32, f32)>,
}

/// What a drawn frame says.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    /// Something moving is in view.
    pub moving: bool,
    /// The floor switcher's chips, for clicks.
    pub chips: Vec<Chip>,
}

/// The surface refused a frame (covered, minimised, out of date): retry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refused;

/// The Floor's backends: Metal, Vulkan or DX12. OpenGL can't read the
/// depth buffer the post pass loads (CI's plain Linux runners offered it
/// and the pipeline failed); without these, there's no adapter.
fn primary(mut d: wgpu::InstanceDescriptor) -> wgpu::InstanceDescriptor {
    d.backends = wgpu::Backends::PRIMARY;
    d
}

/// A GPU instance that can draw into the windows of `display`.
#[must_use]
pub fn instance_for(display: Box<dyn wgpu::wgt::WgpuHasDisplayHandle>) -> wgpu::Instance {
    wgpu::Instance::new(primary(wgpu::InstanceDescriptor::new_with_display_handle(
        display,
    )))
}

/// The surface of `window` (call on the main thread).
///
/// # Errors
///
/// [`Error::Surface`] when the window can't be drawn into.
pub fn surface_for(
    instance: &wgpu::Instance,
    window: impl Into<wgpu::SurfaceTarget<'static>>,
) -> Result<wgpu::Surface<'static>, Error> {
    instance
        .create_surface(window)
        .map_err(|e| Error::Surface(e.to_string()))
}

fn device(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), Error> {
    let ask = |fallback: bool| {
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: surface,
            force_fallback_adapter: fallback,
            ..Default::default()
        }))
    };
    let adapter = ask(false)
        .or_else(|_| ask(true))
        .map_err(|e| Error::NoAdapter(e.to_string()))?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("floor"),
        ..Default::default()
    }))
    .map_err(|e| Error::NoDevice(e.to_string()))?;
    Ok((adapter, device, queue))
}

/// Draws the Floor.
pub struct Renderer {
    gpu: Gpu,
    surface: Option<(wgpu::Surface<'static>, wgpu::SurfaceConfiguration)>,
    moving: Vec<Item>,
}

impl Renderer {
    /// A renderer for a window's `surface` of `size` device pixels.
    ///
    /// # Errors
    ///
    /// [`Error`] when there's no adapter, device or usable surface.
    pub fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        size: (u32, u32),
        scene: &Scene,
    ) -> Result<Self, Error> {
        let (adapter, device, queue) = device(instance, Some(&surface))?;
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| Error::Surface("no format".into()))?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.0.max(1),
            height: size.1.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: Vec::new(),
            color_space: wgpu::wgt::SurfaceColorSpace::default(),
        };
        surface.configure(&device, &config);
        Ok(Self {
            gpu: Gpu::new(device, queue, format, scene),
            surface: Some((surface, config)),
            moving: Vec::with_capacity(20_000),
        })
    }

    /// A renderer that draws only offscreen (the gallery's golden frames).
    ///
    /// # Errors
    ///
    /// [`Error`] when there's no adapter or device.
    pub fn headless(scene: &Scene) -> Result<Self, Error> {
        let instance = wgpu::Instance::new(primary(
            wgpu::InstanceDescriptor::new_without_display_handle(),
        ));
        let (_, device, queue) = device(&instance, None)?;
        Ok(Self {
            gpu: Gpu::new(device, queue, wgpu::TextureFormat::Rgba8Unorm, scene),
            surface: None,
            moving: Vec::new(),
        })
    }

    /// Takes in a rebuilt scene.
    pub fn load(&mut self, scene: &Scene) {
        self.gpu.load(scene);
    }

    /// The window's new size, device pixels.
    pub fn resize(&mut self, w: u32, h: u32) {
        if let Some((surface, config)) = &mut self.surface
            && w > 0
            && h > 0
        {
            config.width = w;
            config.height = h;
            surface.configure(&self.gpu.device, config);
        }
    }

    /// The pick id under a device-pixel position, from the last frame.
    #[must_use]
    pub fn pick(&self, x: f32, y: f32, scale: f32) -> u32 {
        self.gpu.pick(x, y, scale)
    }

    fn frame_into(
        &mut self,
        scene: &Scene,
        view: &View,
        live: &Live,
        size: (u32, u32, f32),
        extras: &Extras<'_>,
        target: &wgpu::TextureView,
    ) -> Shown {
        let floors = view.floors(&scene.frame_size());
        let live = Live {
            eye: view.cam.eye(),
            ..live.clone()
        };
        let drawn = scene.frame(&live, &floors, &mut self.moving);
        let vp = view.cam.vp(size.0 as f32 / size.1.max(1) as f32);
        let over = overlay::Inputs {
            view,
            vp: &vp,
            size: (size.0, size.1),
            scale: size.2,
            marks: &drawn.marks,
            hover: live.hover,
            mouse: extras.mouse,
            alert: extras.alert,
            hint: extras.hint,
        };
        let (quads, chips) = overlay::build(scene, &over);
        let frame = Frame {
            view,
            floors: &floors,
            size,
            t: live.t,
            alert: live.alert,
            hover: live.hover,
            moving: &self.moving,
            lights: &drawn.lights,
            quads: &quads,
        };
        self.gpu.draw(scene, &frame, target);
        Shown {
            moving: drawn.moving,
            chips,
        }
    }

    /// Draws a frame into the window.
    ///
    /// # Errors
    ///
    /// [`Refused`] when the surface gave no frame; draw again shortly.
    pub fn present(
        &mut self,
        scene: &Scene,
        view: &View,
        live: &Live,
        size: (u32, u32, f32),
        extras: &Extras<'_>,
    ) -> Result<Shown, Refused> {
        let Some((surface, config)) = &self.surface else {
            return Err(Refused);
        };
        let texture = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                surface.configure(&self.gpu.device, config);
                return Err(Refused);
            }
            _ => return Err(Refused),
        };
        let target = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let shown = self.frame_into(scene, view, live, size, extras, &target);
        self.gpu.queue.present(texture);
        let _ = self.gpu.device.poll(wgpu::PollType::Poll);
        Ok(shown)
    }

    /// Draws a frame offscreen at `size` and reads it back as RGBA.
    ///
    /// # Errors
    ///
    /// [`Error::Surface`] when the frame can't be read back.
    pub fn capture(
        &mut self,
        scene: &Scene,
        view: &View,
        live: &Live,
        size: (u32, u32, f32),
        extras: &Extras<'_>,
    ) -> Result<Img, Error> {
        let (w, h, _) = size;
        let texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("capture"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.gpu.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.frame_into(
            scene,
            view,
            live,
            size,
            extras,
            &texture.create_view(&wgpu::TextureViewDescriptor::default()),
        );
        let row = (w * 4).div_ceil(256) * 256;
        let buf = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("capture"),
            size: u64::from(row) * u64::from(h),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("capture"),
            });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.gpu.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = self.gpu.device.poll(wgpu::PollType::wait_indefinitely());
        if !rx.recv().is_ok_and(|r| r.is_ok()) {
            return Err(Error::Surface("the frame couldn't be read back".into()));
        }
        let data = slice
            .get_mapped_range()
            .map_err(|e| Error::Surface(e.to_string()))?;
        let bgra = matches!(
            self.gpu.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let mut img = Img::new(w.cast_signed(), h.cast_signed());
        for y in 0..h {
            for x in 0..w {
                let i = (y * row + x * 4) as usize;
                let o = ((y * w + x) * 4) as usize;
                let (r, g, b) = if bgra {
                    (data[i + 2], data[i + 1], data[i])
                } else {
                    (data[i], data[i + 1], data[i + 2])
                };
                img.px[o..o + 4].copy_from_slice(&[r, g, b, 255]);
            }
        }
        Ok(img)
    }
}
