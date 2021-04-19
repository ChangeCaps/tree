#version 450

layout(location = 0) in vec3 Vertex_Position;

void main() {
	gl_Position = vec4(Vertex_Position.xy * 2.0, 0.0, 1.0);
}
