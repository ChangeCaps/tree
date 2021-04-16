#version 450

layout(location = 0) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 0, binding = 0) uniform texture2D SkyPassTexture;
layout(set = 0, binding = 1) uniform sampler SkyPassTextureSampler;

void main() {
    vec3 color = vec3(0.0);

    vec2 texel_size = 1.0 / textureSize(sampler2D(SkyPassTexture, SkyPassTextureSampler), 0).xy;

    const int BLUR = 0;

    for (int x = -BLUR; x <= BLUR; x++) {
        for (int y = -BLUR; y <= BLUR; y++) {
            vec2 offset = vec2(x, y) * texel_size;

            vec4 t = texture(sampler2D(SkyPassTexture, SkyPassTextureSampler), v_Uv + offset);

            color += t.xyz;
        }
    }

    if (BLUR > 0) {
        color /= pow(BLUR * 2 + 1, 2);
    }


    color = color * vec3(1.11, 0.89, 0.79);
    color = 1.35 * color / (1.0 + color);

    o_Target = vec4(color, 1.0);
}