#version 450

layout(location = 0) in vec2 v_Uv;

layout(location = 0) out vec4 o_Target;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 CamProj;
};

layout(set = 0, binding = 1) uniform CameraPosition {
    vec3 CamPos;
};

layout(set = 1, binding = 0) uniform Sun {
    mat4 SunProj;
    vec3 SunPos;
};

layout(set = 2, binding = 0) uniform texture2D SkyPassTexture;
layout(set = 2, binding = 1) uniform sampler SkyPassTextureSampler;

layout(set = 2, binding = 4) uniform texture2D VolumePassTexture;
layout(set = 2, binding = 5) uniform sampler VolumePassTextureSampler;

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

    vec2 texture_size = textureSize(sampler2D(VolumePassTexture, VolumePassTextureSampler), 0).xy;

    const int SUN_BLUR = 2;

    float sun = 0.0;

    for (int x = -SUN_BLUR; x < SUN_BLUR; x++) {
        for (int y = -SUN_BLUR; y < SUN_BLUR; y++) {
            vec2 offset = vec2(x, y) / texture_size;

            float s = texture(sampler2D(VolumePassTexture, VolumePassTextureSampler), v_Uv + offset, 0).x;

            sun += s;
        }
    }

    if (SUN_BLUR > 0) {
        sun /= pow(SUN_BLUR * 2 + 1, 2);
    }

    color = color * vec3(1.11, 0.89, 0.79);
    color = 1.35 * color / (1.0 + color);
    color += vec3(1.0, 0.8, 0.2) * sun * 1.0;
    //color = vec3(z);

    o_Target = vec4(color, 1.0);
}
