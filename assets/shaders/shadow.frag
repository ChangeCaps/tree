#version 450

layout(location = 0) in vec4 v_Pos;
layout(location = 0) out vec4 o_Target;

void main() {
    vec4 v = v_Pos;

	o_Target = v;
}
