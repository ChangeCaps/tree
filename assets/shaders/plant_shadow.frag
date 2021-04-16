#version 450

layout(location = 0) in vec4 v_Pos;
layout(location = 1) in vec3 v_ModelPos;
layout(location = 2) in vec3 v_WorldPos;
layout(location = 3) in float v_Sway;
layout(location = 4) in vec2 v_Uv;
layout(location = 5) in flat uint v_Material;

layout(set = 0, binding = 0) uniform Sun {
	mat4 ViewProj;
	vec3 Pos;
};

layout(set = 2, binding = 1) uniform PlantMaterial_growth {
    float Growth;
};

layout(set = 2, binding = 2) uniform texture2D PlantMaterial_leaf_front;
layout(set = 2, binding = 3) uniform sampler PlantMaterial_leaf_front_sampler;

void main() {
    float dither = length(sin(v_ModelPos * 50.0)) - (Growth - v_Sway) * 4.0 + 0.5;

    if (Growth < v_Sway || dither > 0.9) {
        discard;
    }

    if (v_Material == 1) {
        vec4 tex = texture(sampler2D(PlantMaterial_leaf_front, PlantMaterial_leaf_front_sampler), v_Uv / 1.0);

        if (tex.a < 0.9) {
            discard;
        }
    }

	float far = ViewProj[3][3] - ViewProj[2][3];
	gl_FragDepth = length(v_WorldPos - Pos) / 200.0;
}
