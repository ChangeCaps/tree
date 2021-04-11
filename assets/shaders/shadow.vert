#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 0) out vec4 v_Pos;
layout(location = 1) out vec3 v_WorldPos;


void main() {
    vec4 p = vec4(Vertex_Position, 1.0);
    /*
    gl_Position = p;
    v_Pos = p;
    v_WorldPos = (Model * vec4(Vertex_Position, 1.0)).xyz;
    */
}
