#version 450

layout(location = 0) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 0, binding = 0) uniform texture2D SkyPassTexture;
layout(set = 0, binding = 1) uniform sampler SkyPassTextureSampler;

void main() {
    vec3 color = vec3(0.0);

    vec4 t = texture(sampler2D(SkyPassTexture, SkyPassTextureSampler), v_Uv);

    color = t.xyz;

    color = color * vec3(1.11, 0.89, 0.79);
    color = 1.35 * color / (1.0 + color);

    o_Target = vec4(color, 1.0);
}