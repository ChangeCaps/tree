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

layout(set = 2, binding = 2) uniform texture2DMS SkyPassDepth;
layout(set = 2, binding = 3) uniform sampler SkyPassDepthSampler;

layout(set = 3, binding = 0) uniform texture2D ShadowMapTexture;
layout(set = 3, binding = 1) uniform sampler ShadowMapSampler;

void main() {
    vec2 uv = v_Uv * 2.0 - 1.0;
    uv.y *= -1.0;
    mat4 inverse_proj = inverse(CamProj);
    vec4 near = vec4(uv.x, uv.y, 0.0, 1.0);
    near = inverse_proj * near;
    vec4 far = near + inverse_proj[2];
    near.xyz /= near.w;
    far.xyz /= far.w;
    float z_near = 1.0;
    float z_far = 1000.0;


    vec3 pos = CamPos;
    vec3 dir = far.xyz - near.xyz;
    dir = normalize(dir);

    const int SAMPLES = 400;
    const float MAX_LENGTH = 16.0;
    float step_size = MAX_LENGTH / SAMPLES;

    float sun = 0.0;

    vec2 depth_uv = v_Uv;

    vec2 main_depth_size = textureSize(sampler2DMS(SkyPassDepth, SkyPassDepthSampler)).xy;
    float main_depth = texelFetch(sampler2DMS(SkyPassDepth, SkyPassDepthSampler), ivec2(depth_uv * main_depth_size), 0).x;
    main_depth *= 2.0;
    main_depth -= 1.0;
    float z = (2.0 * z_near) / (z_far + z_near - main_depth * (z_far - z_near));
    z *= z_far - z_near;

    float l = 0.0;

    for (int x = 0; x < SAMPLES; x++) {
        //step_size += 0.001;
        pos += dir * step_size;
        l += step_size;

        vec4 p = (SunProj * vec4(pos, 1.0));
        p.xyz /= p.w;
        p.y *= -1.0;

        float depth = texture(sampler2D(ShadowMapTexture, ShadowMapSampler), p.xy * 0.5 + 0.5).x;
        float dist = length(pos - SunPos) / 200.0;

        if (dist < depth && l < z) {
            //sun += min(1.0 / , 1.0);
            sun += l / MAX_LENGTH;
        }
    }

    sun /= SAMPLES;
    sun = min(sun, 0.1);

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
    color *= 0.5 + sun * 10.0;
    //color = vec3(z);

    o_Target = vec4(color, 1.0);
}