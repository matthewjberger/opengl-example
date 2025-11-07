pub mod support {
    pub mod app;
    pub mod shader;
}

use anyhow::Result;
use egui::MenuBar;
use gl::types::*;
use std::{mem, ptr};
use support::app::App;
use support::shader::ShaderProgram;

pub struct Scene {
    pub model: nalgebra_glm::Mat4,
    pub projection: nalgebra_glm::Mat4,
    pub vao: GLuint,
    pub vbo: GLuint,
    pub ibo: GLuint,
    pub shader_program: ShaderProgram,
    pub mvp_location: GLint,
    pub aspect_ratio: f32,
    pub projection_dirty: bool,
}

impl Scene {
    pub fn new() -> Result<Self> {
        let mut vao = 0;
        let mut vbo = 0;
        let mut ibo = 0;

        unsafe {
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (VERTICES.len() * mem::size_of::<Vertex>()) as GLsizeiptr,
                VERTICES.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );

            gl::GenBuffers(1, &mut ibo);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo);
            gl::BufferData(
                gl::ELEMENT_ARRAY_BUFFER,
                (INDICES.len() * mem::size_of::<u32>()) as GLsizeiptr,
                INDICES.as_ptr() as *const gl::types::GLvoid,
                gl::STATIC_DRAW,
            );

            let stride = mem::size_of::<Vertex>() as GLsizei;
            gl::VertexAttribPointer(0, 4, gl::FLOAT, gl::FALSE, stride, ptr::null());
            gl::EnableVertexAttribArray(0);
            gl::VertexAttribPointer(
                1,
                4,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (4 * mem::size_of::<f32>()) as *const _,
            );
            gl::EnableVertexAttribArray(1);
        }

        let mut shader_program = ShaderProgram::new();
        shader_program
            .vertex_shader("shaders/triangle/triangle.vs.glsl")?
            .fragment_shader("shaders/triangle/triangle.fs.glsl")?
            .link()?;

        let mvp_location = shader_program.uniform_location("mvp");

        Ok(Self {
            model: nalgebra_glm::Mat4::identity(),
            projection: nalgebra_glm::Mat4::identity(),
            vao,
            vbo,
            ibo,
            shader_program,
            mvp_location,
            aspect_ratio: 1.0,
            projection_dirty: true,
        })
    }

    pub fn update(&mut self, delta_time: f32) {
        self.model = nalgebra_glm::rotate(
            &self.model,
            30_f32.to_radians() * delta_time,
            &nalgebra_glm::Vec3::y(),
        );
    }

    pub fn render(&self, _time: f32) {
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);

            gl::ClearColor(0.19, 0.24, 0.42, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        let view = nalgebra_glm::look_at_lh(
            &nalgebra_glm::vec3(0.0, 0.0, 3.0),
            &nalgebra_glm::vec3(0.0, 0.0, 0.0),
            &nalgebra_glm::Vec3::y(),
        );
        let mvp = self.projection * view * self.model;

        self.shader_program.activate();

        unsafe {
            gl::UniformMatrix4fv(self.mvp_location, 1, gl::FALSE, mvp.as_ptr());

            gl::BindVertexArray(self.vao);
            gl::DrawElements(
                gl::TRIANGLES,
                INDICES.len() as _,
                gl::UNSIGNED_INT,
                ptr::null(),
            );
        }
    }

    pub fn set_aspect_ratio(&mut self, width: u32, height: u32) {
        let new_aspect_ratio = width as f32 / height.max(1) as f32;
        if (new_aspect_ratio - self.aspect_ratio).abs() > f32::EPSILON {
            self.aspect_ratio = new_aspect_ratio;
            self.projection_dirty = true;
        }
    }

    pub fn update_projection(&mut self) {
        if self.projection_dirty {
            self.projection = nalgebra_glm::perspective_lh_zo(
                self.aspect_ratio,
                80_f32.to_radians(),
                0.1,
                1000.0,
            );
            self.projection_dirty = false;
        }
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.ibo);
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 4],
    color: [f32; 4],
}

const VERTICES: [Vertex; 3] = [
    Vertex {
        position: [1.0, -1.0, 0.0, 1.0],
        color: [1.0, 0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-1.0, -1.0, 0.0, 1.0],
        color: [0.0, 1.0, 0.0, 1.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0, 1.0],
        color: [0.0, 0.0, 1.0, 1.0],
    },
];

const INDICES: [u32; 3] = [0, 1, 2];

#[derive(Default)]
pub struct TriangleApp {
    scene: Option<Scene>,
}

impl App for TriangleApp {
    fn initialize(&mut self) -> Result<()> {
        self.scene = Some(Scene::new()?);
        Ok(())
    }

    fn update(&mut self, delta_time: f32) -> Result<()> {
        if let Some(scene) = &mut self.scene {
            scene.update(delta_time);
            scene.update_projection();
        }
        Ok(())
    }

    fn render(&mut self, time: f32) -> Result<()> {
        if let Some(scene) = &self.scene {
            scene.render(time);
        }
        Ok(())
    }

    fn render_ui(&mut self, ctx: &egui::Context) -> Result<()> {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("Load").clicked() {
                            ui.close();
                        }
                        if ui.button("Save").clicked() {
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Import").clicked() {
                            ui.close();
                        }
                    });

                    ui.menu_button("Edit", |ui| {
                        if ui.button("Clear").clicked() {
                            ui.close();
                        }
                        if ui.button("Reset").clicked() {
                            ui.close();
                        }
                    });

                    ui.separator();

                    ui.label(egui::RichText::new("Rust/OpenGL").color(egui::Color32::LIGHT_GREEN));

                    ui.separator();
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new("v0.1.0").color(egui::Color32::ORANGE));
                    ui.separator();
                });
            });
        });

        egui::SidePanel::left("left").show(ctx, |ui| {
            ui.heading("Scene Tree");
        });

        egui::SidePanel::right("right").show(ctx, |ui| {
            ui.heading("Inspector");
        });

        egui::TopBottomPanel::bottom("Console").show(ctx, |ui| {
            ui.heading("Console");
        });

        Ok(())
    }

    fn on_resize(&mut self, width: u32, height: u32) -> Result<()> {
        if let Some(scene) = &mut self.scene {
            scene.set_aspect_ratio(width, height);
        }
        Ok(())
    }
}
