#version 450

layout(location = 0) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 View;
};

void main() {
    mat4 inverse_view = inverse(View);
    vec4 near = vec4(v_Uv.x, v_Uv.y, 0.0, 1.0);
    near = inverse_view * near;
    vec4 far = near + inverse_view[2];
    near.xyz /= near.w;
    far.xyz /= far.w;

    vec3 dir = far.xyz - near.xyz;
    dir = normalize(dir);

    float sky = max(dir.y * 0.5 + 0.5, 0.0);

    vec3 color = vec3(0.0);
    color = vec3(0.5, 0.8, 0.9) - sky * 0.5;
    color = mix(color, vec3(0.5, 0.7, 0.9), exp(-10.0 * sky));

    o_Target = vec4(color, 1.0);
}