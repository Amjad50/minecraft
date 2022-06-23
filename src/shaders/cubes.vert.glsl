#version 450

layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec4 color;
layout(location = 3) in vec3 rotation;
layout(location = 4) in vec3 translation;

layout(location = 0) out vec4 v_color;
layout(location = 1) out vec3 v_normal;
layout(location = 2) flat out uint v_selected;
layout(location = 3) flat out uint v_selected2;

layout(set = 0, binding = 0) uniform UniformData {
    mat4 perspective;
    mat4 view;
    vec4 selected;
    vec4 selected2;
} u;

void main() {
    float c1 = cos(rotation.x);
    float s1 = sin(rotation.x);
    float c2 = cos(rotation.y);
    float s2 = sin(rotation.y);
    float c3 = cos(rotation.z);
    float s3 = sin(rotation.z);

    // https://en.wikipedia.org/wiki/Euler_angles#Rotation_matrix
    mat4 object_transform = mat4(
        c2 * c3, c1 * s3 + c3 * s1 * s2, s1 * s3 - c1 * c3 * s2, 0,
        -c2 * s3, c1 * c3 - s1 * s2 * s3, c3 * s1 + c1 * s2 * s3, 0,
        s2, -c2 * s1, c1 * c2, 0,
        translation, 1
    );
    gl_Position = u.perspective * u.view * object_transform * vec4(pos, 1);
    v_selected = uint(u.selected.w == 1.0 && (vec3(u.selected) == translation));
    v_selected2 = uint(u.selected2.w == 1.0 && (vec3(u.selected2) == translation));

    // this transformation only works if scaling is uniform 
    // (scaling of x, y, z by the same value), currently, we don't scale so its ok
    // to compute the value correctly we should use transpose(inverse(mat3(transformation)))
    vec3 normal_world_space = normalize(mat3(object_transform) * normal);

    v_color = color;
    v_normal = normal_world_space;
}
