#version 450

layout(location = 0) in vec3 v_Normal;
layout(location = 1) in flat int v_Material;
layout(location = 2) in vec3 v_WorldPos;

layout(location = 0) out vec4 o_Target;

layout(set = 3, binding = 0) uniform texture2D ShadowTexture;
layout(set = 3, binding = 1) uniform sampler ShadowTexture_sampler;

layout(set = 0, binding = 1) uniform SunCameraViewProj {
	mat4 ViewProj;
};


void main() {
	vec4 projected = ViewProj * vec4(v_WorldPos, 1.0);
	vec4 depth = texture(sampler2D(ShadowTexture, ShadowTexture_sampler), (projected.xy + 1.0) * 0.5);

    vec3 color;

    if (v_Material == 0) {
        color = vec3(155.0 / 255.0, 118.0 / 255.0, 83.0 / 255.0);
    } else {
        color = vec3(0.1, 0.8, 0.2);
    }

    float sun_diffuse = clamp(dot(v_Normal, vec3(1.0, 1.0, 0.0)), 0.01, 1.0);

    color *= sun_diffuse;
	color *= depth.r;

    o_Target = vec4(color, 1.0);
}
