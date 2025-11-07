use anyhow::Result;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextAttributesBuilder, PossiblyCurrentContext, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Theme, Window, WindowAttributes};

#[cfg(debug_assertions)]
use gl::types::*;
#[cfg(debug_assertions)]
use std::ffi::CStr;

pub trait App {
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }
    fn update(&mut self, _delta_time: f32) -> Result<()> {
        Ok(())
    }
    fn render(&mut self, _time: f32) -> Result<()> {
        Ok(())
    }
    fn render_ui(&mut self, _ctx: &egui::Context) -> Result<()> {
        Ok(())
    }
    fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
    fn on_resize(&mut self, _width: u32, _height: u32) -> Result<()> {
        Ok(())
    }
}

struct AppRunner {
    window: Option<Arc<Window>>,
    gl_context: Option<PossiblyCurrentContext>,
    gl_surface: Option<Surface<WindowSurface>>,
    egui_glow: Option<egui_glow::Painter>,
    egui_state: Option<egui_winit::State>,
    egui_ctx: Option<egui::Context>,
    app: Box<dyn App>,
    start_time: Instant,
    last_frame_time: Instant,
}

impl ApplicationHandler for AppRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = WindowAttributes::default()
            .with_title("OpenGL Example")
            .with_inner_size(PhysicalSize::new(800, 600));

        let template = ConfigTemplateBuilder::new();

        let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attributes));

        let (window, gl_config) = display_builder
            .build(event_loop, template, |configs| {
                configs
                    .reduce(|accum, config| {
                        if config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .unwrap()
            })
            .unwrap();

        let window = Arc::new(window.unwrap());

        let gl_display = gl_config.display();

        let context_attributes = ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::OpenGl(Some(Version::new(
                3, 3,
            ))))
            .with_profile(glutin::context::GlProfile::Core)
            .build(Some(window.window_handle().unwrap().as_raw()));

        let gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attributes)
                .unwrap()
        };

        let (width, height) = (window.inner_size().width, window.inner_size().height);

        let attrs = glutin::surface::SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.window_handle().unwrap().as_raw(),
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );

        let gl_surface = unsafe {
            gl_display
                .create_window_surface(&gl_config, &attrs)
                .unwrap()
        };

        let gl_context = gl_context.make_current(&gl_surface).unwrap();

        gl::load_with(|symbol| {
            let symbol = std::ffi::CString::new(symbol).unwrap();
            gl_display.get_proc_address(symbol.as_c_str()).cast()
        });

        enable_gl_debug();

        let glow_context = unsafe {
            glow::Context::from_loader_function(|symbol| {
                let symbol = std::ffi::CString::new(symbol).unwrap();
                gl_display.get_proc_address(symbol.as_c_str()).cast()
            })
        };

        let egui_glow = egui_glow::Painter::new(Arc::new(glow_context), "", None, false).unwrap();

        let egui_ctx = egui::Context::default();
        let viewport_id = egui_ctx.viewport_id();

        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            viewport_id,
            &window,
            Some(window.scale_factor() as _),
            Some(Theme::Dark),
            None,
        );

        if let Err(error) = self.app.initialize() {
            eprintln!("Initialization error: {}", error);
        }

        if let Err(error) = self.app.on_resize(width, height) {
            eprintln!("Resize error: {}", error);
        }

        self.window = Some(window);
        self.gl_context = Some(gl_context);
        self.gl_surface = Some(gl_surface);
        self.egui_glow = Some(egui_glow);
        self.egui_state = Some(egui_state);
        self.egui_ctx = Some(egui_ctx);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };

        let (Some(egui_state), Some(egui_ctx), Some(egui_glow)) = (
            self.egui_state.as_mut(),
            self.egui_ctx.as_ref(),
            self.egui_glow.as_mut(),
        ) else {
            return;
        };

        let event_response = egui_state.on_window_event(window, &event);

        if event_response.consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                if let Err(error) = self.app.cleanup() {
                    eprintln!("Cleanup error: {}", error);
                }
                egui_glow.destroy();
                event_loop.exit();
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                if width == 0 || height == 0 {
                    return;
                }

                if let (Some(gl_context), Some(gl_surface)) =
                    (self.gl_context.as_ref(), self.gl_surface.as_ref())
                {
                    gl_surface.resize(
                        gl_context,
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    );
                }

                unsafe {
                    gl::Viewport(0, 0, width as _, height as _);
                }

                if let Err(error) = self.app.on_resize(width, height) {
                    eprintln!("Resize error: {}", error);
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        ..
                    },
                ..
            } => {
                if matches!(key_code, winit::keyboard::KeyCode::Escape) {
                    if let Err(error) = self.app.cleanup() {
                        eprintln!("Cleanup error: {}", error);
                    }
                    egui_glow.destroy();
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta_time = (now - self.last_frame_time).as_secs_f32();
                let time = (now - self.start_time).as_secs_f32();
                self.last_frame_time = now;

                if let Err(error) = self.app.update(delta_time) {
                    eprintln!("Update error: {}", error);
                }

                if let Err(error) = self.app.render(time) {
                    eprintln!("Render error: {}", error);
                }

                let raw_input = egui_state.take_egui_input(window);
                egui_ctx.begin_pass(raw_input);

                if let Err(error) = self.app.render_ui(egui_ctx) {
                    eprintln!("UI render error: {}", error);
                }

                let egui::FullOutput {
                    platform_output,
                    textures_delta,
                    shapes,
                    pixels_per_point,
                    ..
                } = egui_ctx.end_pass();

                egui_state.handle_platform_output(window, platform_output);

                let clipped_primitives = egui_ctx.tessellate(shapes, pixels_per_point);

                let (width, height) = (window.inner_size().width, window.inner_size().height);

                for (id, image_delta) in textures_delta.set {
                    egui_glow.set_texture(id, &image_delta);
                }

                unsafe {
                    gl::Disable(gl::SCISSOR_TEST);
                }

                egui_glow.paint_primitives([width, height], pixels_per_point, &clipped_primitives);

                for id in textures_delta.free {
                    egui_glow.free_texture(id);
                }

                if let (Some(gl_surface), Some(gl_context)) =
                    (self.gl_surface.as_ref(), self.gl_context.as_ref())
                    && let Err(error) = gl_surface.swap_buffers(gl_context)
                {
                    eprintln!("Swap buffers error: {}", error);
                }

                window.request_redraw();
            }
            _ => (),
        }
    }
}

pub fn run_application(app: impl App + 'static) -> Result<()> {
    env_logger::init();

    let event_loop = EventLoop::builder().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    let now = Instant::now();

    let mut app_runner = AppRunner {
        window: None,
        gl_context: None,
        gl_surface: None,
        egui_glow: None,
        egui_state: None,
        egui_ctx: None,
        app: Box::new(app),
        start_time: now,
        last_frame_time: now,
    };

    event_loop.run_app(&mut app_runner)?;

    Ok(())
}

#[cfg(debug_assertions)]
fn enable_gl_debug() {
    unsafe {
        if gl::DebugMessageCallback::is_loaded() {
            gl::Enable(gl::DEBUG_OUTPUT);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(gl_debug_callback), std::ptr::null());
            gl::DebugMessageControl(
                gl::DONT_CARE,
                gl::DONT_CARE,
                gl::DONT_CARE,
                0,
                std::ptr::null(),
                gl::TRUE,
            );
            log::info!("OpenGL debug layer enabled");
        } else {
            log::warn!("OpenGL debug callbacks not supported");
        }
    }
}

#[cfg(not(debug_assertions))]
fn enable_gl_debug() {}

#[cfg(debug_assertions)]
extern "system" fn gl_debug_callback(
    source: GLenum,
    error_type: GLenum,
    id: GLuint,
    severity: GLenum,
    _length: GLsizei,
    message: *const GLchar,
    _user_param: *mut std::ffi::c_void,
) {
    let message = unsafe {
        if message.is_null() {
            return;
        }
        CStr::from_ptr(message).to_string_lossy()
    };

    let source_str = match source {
        gl::DEBUG_SOURCE_API => "API",
        gl::DEBUG_SOURCE_WINDOW_SYSTEM => "Window System",
        gl::DEBUG_SOURCE_SHADER_COMPILER => "Shader Compiler",
        gl::DEBUG_SOURCE_THIRD_PARTY => "Third Party",
        gl::DEBUG_SOURCE_APPLICATION => "Application",
        gl::DEBUG_SOURCE_OTHER => "Other",
        _ => "Unknown",
    };

    let type_str = match error_type {
        gl::DEBUG_TYPE_ERROR => "Error",
        gl::DEBUG_TYPE_DEPRECATED_BEHAVIOR => "Deprecated",
        gl::DEBUG_TYPE_UNDEFINED_BEHAVIOR => "Undefined Behavior",
        gl::DEBUG_TYPE_PORTABILITY => "Portability",
        gl::DEBUG_TYPE_PERFORMANCE => "Performance",
        gl::DEBUG_TYPE_MARKER => "Marker",
        gl::DEBUG_TYPE_PUSH_GROUP => "Push Group",
        gl::DEBUG_TYPE_POP_GROUP => "Pop Group",
        gl::DEBUG_TYPE_OTHER => "Other",
        _ => "Unknown",
    };

    match severity {
        gl::DEBUG_SEVERITY_HIGH => {
            log::error!("[GL {}] {} ({}): {}", source_str, type_str, id, message);
        }
        gl::DEBUG_SEVERITY_MEDIUM => {
            log::warn!("[GL {}] {} ({}): {}", source_str, type_str, id, message);
        }
        gl::DEBUG_SEVERITY_LOW => {
            log::info!("[GL {}] {} ({}): {}", source_str, type_str, id, message);
        }
        gl::DEBUG_SEVERITY_NOTIFICATION => {
            log::debug!("[GL {}] {} ({}): {}", source_str, type_str, id, message);
        }
        _ => {
            log::trace!("[GL {}] {} ({}): {}", source_str, type_str, id, message);
        }
    }
}
