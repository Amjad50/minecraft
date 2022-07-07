#version 450

layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec4 color;
layout(location = 3) in vec3 translation;

layout(location = 0) out vec4 v_color;
layout(location = 1) out vec3 v_normal;

layout(set = 0, binding = 0) uniform UniformData {
    mat4 rotation_scale;
    mat4 perspective;
    mat4 view;
} u;

void main() {
    mat4 object_transform = u.rotation_scale;
    object_transform[3] = vec4(translation, 1.0);

    gl_Position = u.perspective * u.view * object_transform * vec4(pos, 1);

    // this transformation only works if scaling is uniform 
    // (scaling of x, y, z by the same value), currently, we don't scale so its ok
    // to compute the value correctly we should use transpose(inverse(mat3(transformation)))
    vec3 normal_world_space = normalize(mat3(object_transform) * normal);

    v_color = color;
    v_normal = normal_world_space;
}
