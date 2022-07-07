#version 450

layout(location = 0) in vec3 pos;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec4 color;
layout(location = 3) in vec3 translation;

layout(location = 0) out vec4 v_color;

layout(push_constant) uniform PushConstants {
    uvec2 display_size;
    mat4 rotation_scale;
} pc;

void main() {
    mat4 object_transform = pc.rotation_scale;
    object_transform[3] = vec4(translation, 1.0);

    vec2 p = vec2((object_transform * vec4(pos, 1)));

    p = ((vec2(p) / vec2(pc.display_size)) * 2) - 1;

    gl_Position = vec4(p, 0, 1);

    v_color = color;
}
