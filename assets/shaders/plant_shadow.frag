#version 450

layout(location = 0) in vec4 v_Pos;
layout(location = 1) in vec3 v_ModelPos;
layout(location = 2) in vec3 v_WorldPos;
layout(location = 3) in float v_Sway;

layout(set = 0, binding = 0) uniform Sun {
	mat4 ViewProj;
	vec3 Pos;
};

layout(set = 2, binding = 1) uniform PlantMaterial_growth {
    float Growth;
};

void main() {
    float dither = length(sin(v_ModelPos * 50.0)) - (Growth - v_Sway) * 4.0 + 0.5;

    if (Growth < v_Sway || dither > 0.9) {
        discard;
    }

	float far = ViewProj[3][3] - ViewProj[2][3];
	gl_FragDepth = length(v_WorldPos - Pos) / 200.0;
}
