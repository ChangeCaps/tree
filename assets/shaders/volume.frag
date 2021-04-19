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
    mat4 inverse_proj = inverse(CamProj);
    vec4 near = vec4(v_Uv.x, v_Uv.y, 0.0, 1.0);
    near = inverse_proj * near;
    vec4 far = near + inverse_proj[2];
    near.xyz /= near.w;
    far.xyz /= far.w;
    float z_near = 1.0;
    float z_far = 1000.0;


    vec3 org = CamPos;
    vec3 dir = far.xyz - near.xyz;
    dir = normalize(dir);

    const int SAMPLES = 200;
    const float MAX_LENGTH = 32.0;

    float sun = 0.0;

    vec2 depth_uv = v_Uv * 0.5;
    depth_uv.y *= -1.0;
    depth_uv += 0.5;

    vec2 main_depth_size = textureSize(sampler2DMS(SkyPassDepth, SkyPassDepthSampler)).xy;
    float main_depth = texelFetch(sampler2DMS(SkyPassDepth, SkyPassDepthSampler), ivec2(depth_uv * main_depth_size), 0).x;
    main_depth *= 2.0;
    main_depth -= 1.0;
    float z = (2.0 * z_near) / (z_far + z_near - main_depth * (z_far - z_near));
    z *= z_far - z_near;

	float ray_length = min(z, MAX_LENGTH);
    float sample_density = 1.0 / SAMPLES;

    for (int x = 0; x < SAMPLES; x++) {
        //step_size += 0.001;
        float l = pow(float(x) / float(SAMPLES), 2) * ray_length;
        vec3 pos = org + dir * l;

        vec4 p = (SunProj * vec4(pos, 1.0));
        p.xy /= p.w;
        p.y *= -1.0;

        float depth = texture(sampler2D(ShadowMapTexture, ShadowMapSampler), p.xy * 0.5 + 0.5).x;
        float dist = length(pos - SunPos) / 200.0;

        if (dist < depth) {
            float l = min(1.0, l / MAX_LENGTH * 10.0);
            float d = (depth - dist) * 3.0;
            sun += sample_density * d * l;
        }
    }

    sun = pow(sun, 0.3);

	o_Target = vec4(sun);
}
