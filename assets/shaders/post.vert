#version 450

layout(location = 0) in vec3 Vertex_Position;

layout(location = 0) out vec2 v_Uv;

void main() {
    gl_Position = vec4(Vertex_Position.xy * 2.0, 0.0, 1.0);
    v_Uv = Vertex_Position.xy;
    v_Uv.y *= -1.0;
    v_Uv += 0.5;
}