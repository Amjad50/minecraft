#version 450

layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec4 color;
layout(location = 3) in vec3 rotation;
layout(location = 4) in vec3 translation;
layout(location = 5) in float scale;

layout(location = 0) out vec4 v_color;

layout(push_constant) uniform PushConstants {
    uvec2 display_size;
} pc;

void main() {

    float c1 = cos(rotation.x);
    float s1 = sin(rotation.x);
    float c2 = cos(rotation.y);
    float s2 = sin(rotation.y);
    float c3 = cos(rotation.z);
    float s3 = sin(rotation.z);

    // https://en.wikipedia.org/wiki/Euler_angles#Rotation_matrix
    mat4 object_transform = mat4(
        (c2 * c3) * scale, (c1 * s3 + c3 * s1 * s2), (s1 * s3 - c1 * c3 * s2), 0,
        (-c2 * s3), (c1 * c3 - s1 * s2 * s3) * scale, (c3 * s1 + c1 * s2 * s3), 0,
        (s2), (-c2 * s1), (c1 * c2) * scale, 0,
        translation, 1
    );
    vec2 p = vec2((object_transform * vec4(pos, 1)));

    p = ((vec2(p) / vec2(pc.display_size)) * 2) - 1;

    gl_Position = vec4(p, 0, 1);

    v_color = color;
}
