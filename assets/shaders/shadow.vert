#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 0) out vec4 v_Pos;
layout(location = 1) out vec3 v_WorldPos;

layout(set = 0, binding = 0) uniform Sun {
    mat4 ViewProj;
    vec3 Pos;
};

layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

void main() {
    vec4 p = ViewProj * Model * vec4(Vertex_Position, 1.0);
    gl_Position = p;
    v_Pos = p;
    v_WorldPos = (Model * vec4(Vertex_Position, 1.0)).xyz;
}
