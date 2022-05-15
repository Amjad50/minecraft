#version 450

layout(location = 0) in vec3 center_pos;
layout(location = 1) in vec3 pos;
layout(location = 2) in vec4 color;
layout(location = 3) in vec3 rotation;

layout(location = 0) out vec4 v_color;

layout(push_constant) uniform PushConstantData {
    mat4 transformation;
} pc;

void main() {
    float c1 = cos(rotation.x);
    float s1 = sin(rotation.x);
    float c2 = cos(rotation.y);
    float s2 = sin(rotation.y);
    float c3 = cos(rotation.z);
    float s3 = sin(rotation.z);

    // we translate the vertex to the origin so we can rotate around
    // the center of the object easily
    vec3 origin_position = pos - center_pos;

    // https://en.wikipedia.org/wiki/Euler_angles#Rotation_matrix
    mat4 rotation_and_translation = mat4(
        c2 * c3, c1 * s3 + c3 * s1 * s2, s1 * s3 - c1 * c3 * s2, 0,
        -c2 * s3, c1 * c3 - s1 * s2 * s3, c3 * s1 + c1 * s2 * s3, 0,
        s2, -c2 * s1, c1 * c2, 0,
        // the center pos here is the amount of translation from the origin
        center_pos, 1
    );

    gl_Position = pc.transformation * rotation_and_translation * vec4(origin_position, 1);

    v_color = color;
}
