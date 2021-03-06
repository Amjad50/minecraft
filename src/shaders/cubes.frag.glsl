#version 450

layout(location = 0) in  vec4 v_color;
layout(location = 1) in  vec3 v_normal;

layout(location = 0) out vec4 f_color;

const vec3 DIRECTION_TO_LIGHT = normalize(vec3(1.0, 3.0, -2.0));
const float AMBIENT_LIGHT = 0.2;

void main() {
    float light_intensity = max(dot(normalize(v_normal), DIRECTION_TO_LIGHT), 0);

    f_color = vec4(v_color.rgb * (light_intensity + AMBIENT_LIGHT), v_color.a);
}
