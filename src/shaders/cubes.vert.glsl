#version 450

layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec4 color;
layout(location = 3) in vec3 translation;

layout(location = 0) out vec4 v_color;
layout(location = 1) out vec3 v_normal;

layout(set = 0, binding = 0) uniform UniformData {
    vec3 rotation;
    float scale;
    mat4 perspective;
    mat4 view;
} u;

void main() {
    float c1 = cos(u.rotation.x);
    float s1 = sin(u.rotation.x);
    float c2 = cos(u.rotation.y);
    float s2 = sin(u.rotation.y);
    float c3 = cos(u.rotation.z);
    float s3 = sin(u.rotation.z);

    // https://en.wikipedia.org/wiki/Euler_angles#Rotation_matrix
    mat4 object_transform = mat4(
        (c2 * c3) * u.scale, (c1 * s3 + c3 * s1 * s2), (s1 * s3 - c1 * c3 * s2), 0,
        (-c2 * s3), (c1 * c3 - s1 * s2 * s3) * u.scale, (c3 * s1 + c1 * s2 * s3), 0,
        (s2), (-c2 * s1), (c1 * c2) * u.scale, 0,
        translation, 1
    );
    gl_Position = u.perspective * u.view * object_transform * vec4(pos, 1);

    // this transformation only works if scaling is uniform 
    // (scaling of x, y, z by the same value), currently, we don't scale so its ok
    // to compute the value correctly we should use transpose(inverse(mat3(transformation)))
    vec3 normal_world_space = normalize(mat3(object_transform) * normal);

    v_color = color;
    v_normal = normal_world_space;
}
