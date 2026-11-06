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
//!
//! Must not know: storage, tools or Tauri: it draws what juggler-floor
//! describes.

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

/// How the office is lit and finished (0.4.1, `docs/research/floor-04x.md`
/// §6–7): the sky (day and night from the Mac's clock, or a fixed one)
/// and whether the reworked look is on (ambient occlusion, soft shadows,
/// a rim light, the grade, dust on what's long unused), with its second
/// pass. The default is 0.4.1's look (D-86): reworked, materials and
/// faces, the finer Pixel art; [`Look::classic`] is 0.4.0's, kept for
/// rollback and its tests.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// The light.
    pub sky: juggler_floor::daylight::Sky,
    /// The reworked look.
    pub rework: bool,
    /// The second pass (`docs/research/floor-04x.md` §12): every part's
    /// material (generated maps, normals and roughness), highlights and
    /// faces. Its maps are made the first time it's drawn.
    pub materials: bool,
    /// Pixel art's finer pixel (two device pixels on a Retina screen,
    /// not three).
    pub fine: bool,
}

impl Default for Look {
    fn default() -> Self {
        Self {
            sky: juggler_floor::daylight::Sky::always_day(),
            rework: true,
            materials: true,
            fine: true,
        }
    }
}

impl Look {
    /// 0.4.0's look: flat colours, the classic light, Pixel art's
    /// coarser pixel (for rollback and its tests).
    #[must_use]
    pub fn classic() -> Self {
        Self {
            sky: juggler_floor::daylight::Sky::classic(),
            rework: false,
            materials: false,
            fine: false,
        }
    }
}

/// What else a frame shows besides the scene.
#[derive(Clone, Copy, Debug)]
pub struct Extras<'a> {
    /// How to move the view, in this system's words.
    pub hint: &'a str,
    /// The overload alert's words, while it's on.
    pub alert: Option<&'a str>,
    /// The pointer, device pixels.
    pub mouse: Option<(f32, f32)>,
    /// AJ-E3517: the hover card open now.
    pub card: Option<&'a overlay::Card>,
}

/// What a drawn frame says.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    /// Something moving is in view.
    pub moving: bool,
    /// The floor switcher's chips, for clicks.
    pub chips: Vec<Chip>,
    /// AJ-E3517: where the hover card went and its buttons, for clicks.
    pub card: Option<overlay::CardOut>,
    /// AJ-E3519: the View button, for clicks.
    pub view: Option<[f32; 4]>,
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
    /// Draw the overlay (off only for the design proposal's bare frames).
    overlay: bool,
    /// AJ-E3515: the zoom band drawn last (its hysteresis), and which
    /// labels show.
    band: Option<juggler_floor::flags::Band>,
    labels: juggler_floor::flags::Labels,
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
            overlay: true,
            band: None,
            labels: juggler_floor::flags::Labels::Smart,
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
            overlay: true,
            band: None,
            labels: juggler_floor::flags::Labels::Smart,
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

    /// D-80: Full resolution (`true`) or Pixel art, from the next frame.
    pub fn set_full(&mut self, full: bool) {
        self.gpu.full = full;
    }

    /// Whether frames carry the overlay (the floor tabs, chips, balloons,
    /// tooltip); off only to draw the design proposal's bare scenes.
    pub fn set_overlay(&mut self, on: bool) {
        self.overlay = on;
    }

    /// AJ-E3515: which labels show, from the next frame.
    pub fn set_labels(&mut self, labels: juggler_floor::flags::Labels) {
        self.labels = labels;
    }

    /// The light and look, from the next frame.
    pub fn set_look(&mut self, look: Look) {
        self.gpu.look = look;
        self.gpu.ensure_materials();
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
        let (vp, eye) = view.sight(&scene.frame_size(), size.0 as f32 / size.1.max(1) as f32);
        let live = Live {
            eye,
            sun: self.gpu.look.sky.sun_dir,
            night: self.gpu.look.sky.night,
            ..live.clone()
        };
        let drawn = scene.frame(&live, &floors, &mut self.moving);
        let band = juggler_floor::flags::band(
            overlay::desk_px(view, &vp, (size.0, size.1), size.2),
            view.focus.is_none(),
            self.band,
        );
        self.band = Some(band);
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
            band,
            labels: self.labels,
            card: extras.card,
        };
        let (mut quads, chips, card, view_button) = overlay::build(scene, &over);
        if !self.overlay {
            quads.clear();
        }
        // bare frames (the proposal's) show a hovered sitter turning to
        // look, without the hover's ring
        let ring = if self.overlay { live.hover } else { 0 };
        let frame = Frame {
            view,
            floors: &floors,
            size,
            t: live.t,
            alert: live.alert,
            hover: ring,
            moving: &self.moving,
            lights: &drawn.lights,
            quads: &quads,
        };
        self.gpu.draw(scene, &frame, target);
        Shown {
            moving: drawn.moving,
            chips,
            card,
            view: view_button,
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

    /// Draws `n` frames offscreen at `size` and waits for the GPU: the
    /// milliseconds a frame took, CPU and GPU together (for the budgets'
    /// records and the design proposal's costs).
    pub fn bench(
        &mut self,
        scene: &Scene,
        view: &View,
        live: &Live,
        size: (u32, u32, f32),
        n: u32,
    ) -> f64 {
        let (w, h, _) = size;
        let texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bench"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.gpu.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let target = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let extras = Extras {
            hint: "",
            alert: None,
            mouse: None,
            card: None,
        };
        // one frame to settle the targets' sizes
        self.frame_into(scene, view, live, size, &extras, &target);
        let _ = self.gpu.device.poll(wgpu::PollType::wait_indefinitely());
        let start = std::time::Instant::now();
        for k in 0..n {
            let l = Live {
                t: live.t + k as f32 / 60.0,
                ..live.clone()
            };
            self.frame_into(scene, view, &l, size, &extras, &target);
        }
        let _ = self.gpu.device.poll(wgpu::PollType::wait_indefinitely());
        start.elapsed().as_secs_f64() * 1000.0 / f64::from(n.max(1))
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
