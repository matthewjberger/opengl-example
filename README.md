# Rust / Winit / Egui / OpenGL Triangle

This project demonstrates how to setup a [rust](https://www.rust-lang.org/) project
that uses [OpenGL](https://www.opengl.org/) via [gl-rs](https://github.com/brendanzab/gl-rs)
and [glutin](https://github.com/rust-windowing/glutin) to render a spinning triangle with
an [egui](https://www.egui.rs/) UI overlay.

> If you're looking for a Vulkan example, check out [the vulkan-example repo](https://github.com/matthewjberger/vulkan-example)

<img width="802" height="632" alt="native" src="https://github.com/user-attachments/assets/aaad05db-8a5b-4306-a166-2692b4e365fb" />

## Quickstart

```bash
cargo run -r
```

## Architecture

The project follows a modular architecture:

- **Support Library** (`src/support/`)
  - `app.rs` - Application framework with event loop and egui integration
  - `shader.rs` - OpenGL shader compilation and program management

- **Main Application** (`src/lib.rs`)
  - Scene management with OpenGL VAO/VBO/IBO
  - Matrix transformations with nalgebra-glm
  - egui UI panels

- **Shaders** (`assets/shaders/`)
  - GLSL vertex and fragment shaders

## Features

- Native OpenGL rendering
- egui UI with OpenGL backend (egui_glow)
- Rotating 3D triangle with perspective projection
- Full UI with menu bar, side panels, and console
- Shader-based rendering pipeline
