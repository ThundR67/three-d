use crate::geometries::mesh::Mesh;
use crate::objects::MeshShader;
use crate::object::*;
use crate::buffer::*;
use crate::core::Gl;

impl Mesh {
    pub fn new_from_obj_source(gl: &Gl, source: String) -> Result<Vec<Object>, Error>
    {
        let objs = wavefront_obj::obj::parse(source).unwrap();

        let mut objects = Vec::new();
        for obj in objs.objects.iter() { // Objects consisting of several meshes with different materials
            let mut object = Object::new(obj.name.to_owned());

            let mut positions = Vec::new();
            obj.vertices.iter().for_each(|v| {
                positions.push(v.x as f32);
                positions.push(v.y as f32);
                positions.push(v.z as f32);
            });
            let mut normals = vec![0.0f32; positions.len() * 3];
            let mut indices = Vec::new();
            let has_normals = !obj.normals.is_empty();

            for mesh in obj.geometry.iter() { // All meshes with different materials
                for primitive in mesh.shapes.iter() { // All triangles with same material
                    match primitive.primitive {
                        wavefront_obj::obj::Primitive::Triangle(i0, i1, i2) => {
                            indices.push(i0.0 as u32);
                            indices.push(i1.0 as u32);
                            indices.push(i2.0 as u32);

                            if has_normals {
                                let mut normal = obj.normals[i0.2.unwrap()];
                                normals[i0.0 * 3] = normal.x as f32;
                                normals[i0.0 * 3 + 1] = normal.y as f32;
                                normals[i0.0 * 3 + 2] = normal.z as f32;

                                normal = obj.normals[i1.2.unwrap()];
                                normals[i1.0 * 3] = normal.x as f32;
                                normals[i1.0 * 3 + 1] = normal.y as f32;
                                normals[i1.0 * 3 + 2] = normal.z as f32;

                                normal = obj.normals[i2.2.unwrap()];
                                normals[i2.0 * 3] = normal.x as f32;
                                normals[i2.0 * 3 + 1] = normal.y as f32;
                                normals[i2.0 * 3 + 2] = normal.z as f32;
                            }
                        },
                        _ => {}
                    }
                }


                let mesh = if !has_normals {
                    Self::new_with_computed_normals(&gl, &indices, &positions)?
                } else {
                    Self::new(&gl, &indices, &positions, &normals)?
                };
                object.add(mesh, MeshShader::new(gl).unwrap());

            }

            objects.push(object);
        }

        Ok(objects)
    }
}