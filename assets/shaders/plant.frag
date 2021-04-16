#version 450

layout(location = 0) in vec3 v_Normal;
layout(location = 1) in vec3 v_Color;
layout(location = 2) in vec3 v_ModelPos;
layout(location = 3) in vec3 v_WorldPos;
layout(location = 4) in vec4 v_ShadowCoord;
layout(location = 5) in float v_Sway;
layout(location = 6) in vec2 v_Uv;
layout(location = 7) in flat uint v_Material;

layout(location = 0) out vec4 o_Target;

layout(set = 0, binding = 1) uniform Sun {
    mat4 SunViewProj;
    vec3 SunPos;
};

layout(set = 2, binding = 1) uniform PlantMaterial_growth {
    float Growth;
};

layout(set = 2, binding = 2) uniform texture2D PlantMaterial_texture;
layout(set = 2, binding = 3) uniform sampler PlantMaterial_texture_sampler;

layout(set = 2, binding = 4) uniform texture2D PlantMaterial_leaf_front;
layout(set = 2, binding = 5) uniform sampler PlantMaterial_leaf_front_sampler;

layout(set = 3, binding = 0) uniform texture2D ShadowMapTexture;
layout(set = 3, binding = 1) uniform sampler ShadowMapSampler;

float calculateShadow(in vec2 uv, in float dist, in float bias) {
    float depth = texture(sampler2D(ShadowMapTexture, ShadowMapSampler), uv).x;

    if (dist - bias < depth || uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
        return 0.0;
    } else {
        return 1.0;
    }
}

void main() {
    float dither = length(sin(v_ModelPos * 50.0)) - (Growth - v_Sway) * 4.0 + 0.5;

    if (Growth < v_Sway || dither > 0.9) {
        discard;
    }

    vec3 s = v_ShadowCoord.xyz / v_ShadowCoord.w;
    s.y *= -1.0;

    vec3 world_to_sun = SunPos - v_WorldPos;

    float far = SunViewProj[3][3] - SunViewProj[2][3];

    float dist = length(world_to_sun) / 200.0;

    vec2 texel_size = 1.0 / textureSize(sampler2D(ShadowMapTexture, ShadowMapSampler), 0);

    float bias = max(0.05 * (1.0 - dot(v_Normal, world_to_sun)), 0.00001);

    const int BLUR = 3;

    float shadow = 0.0;

    for (int x = -BLUR; x <= BLUR; x++) {
        for (int y = -BLUR; y <= BLUR; y++) {
            vec2 offset = vec2(x, y) * texel_size;

            shadow += calculateShadow(s.xy * 0.5 + 0.5 + offset, dist, bias);
        }
    }

    shadow /= pow(BLUR * 2 + 1, 2);

    float sun_diffuse = clamp(dot(v_Normal, normalize(world_to_sun)), 0.0, 1.0);
    float sky_diffuse = sqrt(clamp(0.5 + 0.5 * v_Normal.y, 0.0, 1.0));
    float bounce_diffuse = sqrt(clamp(0.1 - 0.9 * v_Normal.y, 0.0, 1.0)) * clamp(1.0 - 0.1 * v_WorldPos.y, 0.0, 1.0);

    vec3 light = vec3(0.0);

    light += vec3(8.1, 6.0, 4.2) * (1.0 - shadow) * sun_diffuse * 0.2;
    light += vec3(0.5, 0.7, 1.0) * sky_diffuse;


    vec3 color = v_Color;

    if (v_Material == 0) {
        vec2 uv = v_Uv / 1.0;
        uv.y = mod(uv.y, 1.0);

        vec4 tex = texture(sampler2D(PlantMaterial_texture, PlantMaterial_texture_sampler), uv);

        color *= tex.rgb;
    }

    if (v_Material == 1) {
        vec4 tex = texture(sampler2D(PlantMaterial_leaf_front, PlantMaterial_leaf_front_sampler), v_Uv / 1.0);

        if (tex.a < 0.9) {
            discard;
        }

        color *= tex.rgb;
    }

    color = color * light;

    o_Target = vec4(color, 1.0);
}
