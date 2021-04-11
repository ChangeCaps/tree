#version 450

layout(location = 0) in vec4 v_Pos;
layout(location = 1) in vec3 v_WorldPos;
layout(location = 0) out vec4 o_Target;

void main() {
	//gl_FragDepth = length(v_WorldPos - Pos) / 1000.0;
}
