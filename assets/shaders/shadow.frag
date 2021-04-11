#version 450

layout(location = 0) in vec4 v_Pos;
layout(location = 1) in vec3 v_WorldPos;

layout(set = 0, binding = 0) uniform Sun {
	mat4 ViewProj;
	vec3 Pos;
};

void main() {
	float far = ViewProj[3][3] - ViewProj[2][3];
	gl_FragDepth = length(v_WorldPos - Pos) / 200.0;
}
