use crate::core::*;
use crate::renderer::*;
use std::collections::HashMap;

///
/// Used for defining the attributes for each particle in [Particles], for example its starting position and velocity.
/// Each list of attributes must contain the same number of elements as the number of particles.
///
#[derive(Clone, Debug, Default)]
pub struct ParticleData {
    /// Initial positions of each particle in world coordinates.
    pub start_positions: Vec<Vec3>,
    /// Initial velocities of each particle defined in the world coordinate system.
    pub start_velocities: Vec<Vec3>,
    /// The texture transform applied to the uv coordinates of each particle.
    pub texture_transforms: Option<Vec<Mat3>>,
    /// A custom color for each particle.
    pub colors: Option<Vec<Color>>,
}

impl ParticleData {
    ///
    /// Returns an error if the particle data is not valid.
    ///
    pub fn validate(&self) -> Result<(), RendererError> {
        let instance_count = self.count();
        let buffer_check = |length: Option<usize>, name: &str| -> Result<(), RendererError> {
            if let Some(length) = length {
                if length < instance_count as usize {
                    Err(RendererError::InvalidBufferLength(
                        name.to_string(),
                        instance_count as usize,
                        length,
                    ))?;
                }
            }
            Ok(())
        };

        buffer_check(
            self.texture_transforms.as_ref().map(|b| b.len()),
            "texture transforms",
        )?;
        buffer_check(self.colors.as_ref().map(|b| b.len()), "colors")?;
        buffer_check(Some(self.start_positions.len()), "start_positions")?;
        buffer_check(Some(self.start_velocities.len()), "start_velocities")?;

        Ok(())
    }

    /// Returns the number of instances.
    pub fn count(&self) -> u32 {
        self.start_positions.len() as u32
    }
}

///
/// Particle effect that can be rendered with any material.
///
/// All particles are initialised with [ParticleData::start_positions] and [ParticleData::start_velocities] and a global [Particles::acceleration].
/// Then, when time passes, their position is updated based on
///
/// ```no_rust
/// new_position = start_position + start_velocity * time + 0.5 * acceleration * time * time
/// ```
///
/// The particles will therefore only move if the [Particles::time] variable is updated every frame.
///
pub struct Particles {
    context: Context,
    vertex_buffers: HashMap<String, VertexBuffer>,
    instance_buffers: HashMap<String, InstanceBuffer>,
    index_buffer: Option<ElementBuffer>,
    /// The acceleration applied to all particles defined in the world coordinate system. Default is gravity.
    pub acceleration: Vec3,
    instance_count: u32,
    transformation: Mat4,
    texture_transform: Mat3,
    /// A time variable that should be updated each frame.
    pub time: f32,
}

impl Particles {
    ///
    /// Creates a new set of particles with geometry defined by the given cpu mesh.
    ///
    pub fn new(context: &Context, data: &ParticleData, cpu_mesh: &CpuMesh) -> Self {
        #[cfg(debug_assertions)]
        cpu_mesh.validate().expect("invalid cpu mesh");

        let mut particles = Self {
            context: context.clone(),
            index_buffer: super::index_buffer_from_mesh(context, cpu_mesh),
            vertex_buffers: super::vertex_buffers_from_mesh(context, cpu_mesh),
            instance_buffers: HashMap::new(),
            acceleration: vec3(0.0, -9.82, 0.0),
            instance_count: 0,
            transformation: Mat4::identity(),
            texture_transform: Mat3::identity(),
            time: 0.0,
        };
        particles.update(data);
        particles
    }

    ///
    /// Returns local to world transformation applied to the particle geometry before its position is updated as described in [Particles].
    ///
    pub fn transformation(&self) -> Mat4 {
        self.transformation
    }

    ///
    /// Set the local to world transformation applied to the particle geometry before its position is updated as described in [Particles].
    ///
    pub fn set_transformation(&mut self, transformation: Mat4) {
        self.transformation = transformation;
    }

    ///
    /// Get the texture transform applied to the uv coordinates of all of the particles.
    ///
    pub fn texture_transform(&self) -> &Mat3 {
        &self.texture_transform
    }

    ///
    /// Set the texture transform applied to the uv coordinates of all of the particles.
    /// This is applied before the texture transform for each particle.
    ///
    pub fn set_texture_transform(&mut self, texture_transform: Mat3) {
        self.texture_transform = texture_transform;
    }

    ///
    /// Updates the particles with the given data.
    ///
    pub fn update(&mut self, data: &ParticleData) {
        #[cfg(debug_assertions)]
        data.validate().expect("invalid particle data");
        self.instance_count = data.count();
        self.instance_buffers.clear();

        self.instance_buffers.insert(
            "start_position".to_string(),
            InstanceBuffer::new_with_data(&self.context, &data.start_positions),
        );
        self.instance_buffers.insert(
            "start_velocity".to_string(),
            InstanceBuffer::new_with_data(&self.context, &data.start_velocities),
        );
        if let Some(texture_transforms) = &data.texture_transforms {
            let mut instance_tex_transform1 = Vec::new();
            let mut instance_tex_transform2 = Vec::new();
            for texture_transform in texture_transforms.iter() {
                instance_tex_transform1.push(vec3(
                    texture_transform.x.x,
                    texture_transform.y.x,
                    texture_transform.z.x,
                ));
                instance_tex_transform2.push(vec3(
                    texture_transform.x.y,
                    texture_transform.y.y,
                    texture_transform.z.y,
                ));
            }
            self.instance_buffers.insert(
                "tex_transform_row1".to_string(),
                InstanceBuffer::new_with_data(&self.context, &instance_tex_transform1),
            );
            self.instance_buffers.insert(
                "tex_transform_row2".to_string(),
                InstanceBuffer::new_with_data(&self.context, &instance_tex_transform2),
            );
        }
        if let Some(instance_colors) = &data.colors {
            self.instance_buffers.insert(
                "instance_color".to_string(),
                InstanceBuffer::new_with_data(&self.context, &instance_colors),
            );
        }
    }

    fn vertex_shader_source(&self, fragment_shader_source: &str) -> String {
        let use_positions = fragment_shader_source.find("in vec3 pos;").is_some();
        let use_normals = fragment_shader_source.find("in vec3 nor;").is_some();
        let use_tangents = fragment_shader_source.find("in vec3 tang;").is_some();
        let use_uvs = fragment_shader_source.find("in vec2 uvs;").is_some();
        let use_colors = fragment_shader_source.find("in vec4 col;").is_some();
        format!(
            "#define PARTICLES\n{}{}{}{}{}{}{}{}",
            if use_positions {
                "#define USE_POSITIONS\n"
            } else {
                ""
            },
            if use_normals {
                "#define USE_NORMALS\n"
            } else {
                ""
            },
            if use_tangents {
                if fragment_shader_source.find("in vec3 bitang;").is_none() {
                    panic!("if the fragment shader defined 'in vec3 tang' it also needs to define 'in vec3 bitang'");
                }
                "#define USE_TANGENTS\n"
            } else {
                ""
            },
            if use_uvs { "#define USE_UVS\n" } else { "" },
            if use_colors {
                if self.instance_buffers.contains_key("instance_color")
                    && self.vertex_buffers.contains_key("color")
                {
                    "#define USE_COLORS\n#define USE_VERTEX_COLORS\n#define USE_INSTANCE_COLORS\n"
                } else if self.instance_buffers.contains_key("instance_color") {
                    "#define USE_COLORS\n#define USE_INSTANCE_COLORS\n"
                } else {
                    "#define USE_COLORS\n#define USE_VERTEX_COLORS\n"
                }
            } else {
                ""
            },
            if self.instance_buffers.contains_key("tex_transform_row1") {
                "#define USE_INSTANCE_TEXTURE_TRANSFORMATION\n"
            } else {
                ""
            },
            include_str!("../../core/shared.frag"),
            include_str!("shaders/mesh.vert"),
        )
    }
}

impl Geometry for Particles {
    fn aabb(&self) -> AxisAlignedBoundingBox {
        AxisAlignedBoundingBox::INFINITE
    }

    fn render_with_material(
        &self,
        material: &dyn Material,
        camera: &Camera,
        lights: &[&dyn Light],
    ) {
        let fragment_shader_source = material.fragment_shader_source(
            self.vertex_buffers.contains_key("color")
                || self.instance_buffers.contains_key("instance_color"),
            lights,
        );
        self.context.program(
            &self.vertex_shader_source(&fragment_shader_source),
            &fragment_shader_source,
            |program| {
                material.use_uniforms(program, camera, lights);
                program.use_uniform("viewProjection", camera.projection() * camera.view());
                program.use_uniform("modelMatrix", &self.transformation);
                program.use_uniform("acceleration", &self.acceleration);
                program.use_uniform("time", &self.time);
                program.use_uniform_if_required("textureTransform", &self.texture_transform);
                program.use_uniform_if_required(
                    "normalMatrix",
                    &self.transformation.invert().unwrap().transpose(),
                );

                for attribute_name in ["position", "normal", "tangent", "color", "uv_coordinates"] {
                    if program.requires_attribute(attribute_name) {
                        program.use_vertex_attribute(
                            attribute_name,
                            self.vertex_buffers
                                .get(attribute_name).expect(&format!("the render call requires the {} vertex buffer which is missing on the given geometry", attribute_name))
                        );
                    }
                }

                for attribute_name in [
                    "start_position",
                    "start_velocity",
                    "tex_transform_row1",
                    "tex_transform_row2",
                    "instance_color",
                ] {
                    if program.requires_attribute(attribute_name) {
                        program.use_instance_attribute(
                            attribute_name,
                            self.instance_buffers
                            .get(attribute_name).expect(&format!("the render call requires the {} instance buffer which is missing on the given geometry", attribute_name))
                        );
                    }
                }

                if let Some(ref index_buffer) = self.index_buffer {
                    program.draw_elements_instanced(
                        material.render_states(),
                        camera.viewport(),
                        index_buffer,
                        self.instance_count,
                    )
                } else {
                    program.draw_arrays_instanced(
                        material.render_states(),
                        camera.viewport(),
                        self.vertex_buffers.get("position").unwrap().vertex_count() as u32,
                        self.instance_count,
                    )
                }
            },
        ).expect("Failed compiling shader")
    }
}
