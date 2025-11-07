use anyhow::{Result, anyhow};
pub use gl::types::*;
use std::ffi::CString;
use std::{fs, ptr};

pub enum ShaderKind {
    Vertex,
    Fragment,
    Geometry,
    TessellationControl,
    TessellationEvaluation,
    Compute,
}

#[derive(Default)]
pub struct Shader {
    pub id: GLuint,
}

impl Shader {
    pub fn new(shader_type: ShaderKind) -> Shader {
        Shader {
            id: unsafe { gl::CreateShader(Shader::map_type(&shader_type)) },
        }
    }

    pub fn load_file(&mut self, path: &str) -> Result<()> {
        let source = fs::read_to_string(path)
            .map_err(|error| anyhow!("Failed to read shader file '{}': {}", path, error))?;
        self.load(&source)
    }

    pub fn load(&mut self, source: &str) -> Result<()> {
        let source_str = CString::new(source.as_bytes())
            .map_err(|error| anyhow!("Shader source contains null byte: {}", error))?;

        unsafe {
            gl::ShaderSource(self.id, 1, &source_str.as_ptr(), ptr::null());
            gl::CompileShader(self.id);
        }

        self.check_compile_status()
    }

    fn check_compile_status(&self) -> Result<()> {
        let mut success = 0;
        unsafe {
            gl::GetShaderiv(self.id, gl::COMPILE_STATUS, &mut success);
            if success == 0 {
                let mut length = 0;
                gl::GetShaderiv(self.id, gl::INFO_LOG_LENGTH, &mut length);

                if length > 0 {
                    let mut buffer = vec![0u8; length as usize];
                    gl::GetShaderInfoLog(
                        self.id,
                        length,
                        ptr::null_mut(),
                        buffer.as_mut_ptr() as *mut GLchar,
                    );
                    let error_message = String::from_utf8_lossy(&buffer[..length as usize - 1]);
                    return Err(anyhow!("Shader compilation failed:\n{}", error_message));
                }

                return Err(anyhow!("Shader compilation failed with no error message"));
            }
        }
        Ok(())
    }

    fn map_type(shader_type: &ShaderKind) -> GLuint {
        match shader_type {
            ShaderKind::Vertex => gl::VERTEX_SHADER,
            ShaderKind::Fragment => gl::FRAGMENT_SHADER,
            ShaderKind::Geometry => gl::GEOMETRY_SHADER,
            ShaderKind::TessellationControl => gl::TESS_CONTROL_SHADER,
            ShaderKind::TessellationEvaluation => gl::TESS_EVALUATION_SHADER,
            ShaderKind::Compute => gl::COMPUTE_SHADER,
        }
    }
}

#[derive(Default)]
pub struct ShaderProgram {
    pub id: GLuint,
    pub shader_ids: Vec<GLuint>,
}

impl ShaderProgram {
    pub fn new() -> Self {
        ShaderProgram {
            id: unsafe { gl::CreateProgram() },
            shader_ids: Vec::new(),
        }
    }

    fn attach(&mut self, kind: ShaderKind, path: &str) -> Result<&mut Self> {
        let mut shader = Shader::new(kind);
        shader.load_file(path)?;
        unsafe {
            gl::AttachShader(self.id, shader.id);
        }
        self.shader_ids.push(shader.id);
        Ok(self)
    }

    pub fn vertex_shader(&mut self, path: &str) -> Result<&mut Self> {
        self.attach(ShaderKind::Vertex, path)
    }

    pub fn geometry_shader(&mut self, path: &str) -> Result<&mut Self> {
        self.attach(ShaderKind::Geometry, path)
    }

    pub fn tessellation_control_shader(&mut self, path: &str) -> Result<&mut Self> {
        self.attach(ShaderKind::TessellationControl, path)
    }

    pub fn tessellation_evaluation_shader(&mut self, path: &str) -> Result<&mut Self> {
        self.attach(ShaderKind::TessellationEvaluation, path)
    }

    pub fn compute_shader(&mut self, path: &str) -> Result<&mut Self> {
        self.attach(ShaderKind::Compute, path)
    }

    pub fn fragment_shader(&mut self, path: &str) -> Result<&mut Self> {
        self.attach(ShaderKind::Fragment, path)
    }

    pub fn link(&mut self) -> Result<()> {
        unsafe {
            gl::LinkProgram(self.id);
        }

        self.check_link_status()?;

        unsafe {
            for id in &self.shader_ids {
                gl::DeleteShader(*id);
            }
        }
        self.shader_ids.clear();

        Ok(())
    }

    fn check_link_status(&self) -> Result<()> {
        let mut success = 0;
        unsafe {
            gl::GetProgramiv(self.id, gl::LINK_STATUS, &mut success);
            if success == 0 {
                let mut length = 0;
                gl::GetProgramiv(self.id, gl::INFO_LOG_LENGTH, &mut length);

                if length > 0 {
                    let mut buffer = vec![0u8; length as usize];
                    gl::GetProgramInfoLog(
                        self.id,
                        length,
                        ptr::null_mut(),
                        buffer.as_mut_ptr() as *mut GLchar,
                    );
                    let error_message = String::from_utf8_lossy(&buffer[..length as usize - 1]);
                    return Err(anyhow!("Shader program linking failed:\n{}", error_message));
                }

                return Err(anyhow!(
                    "Shader program linking failed with no error message"
                ));
            }
        }
        Ok(())
    }

    pub fn activate(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn uniform_location(&self, name: &str) -> GLint {
        let name: CString = CString::new(name.as_bytes()).unwrap();
        unsafe { gl::GetUniformLocation(self.id, name.as_ptr()) }
    }
}

impl Drop for ShaderProgram {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id) }
    }
}
