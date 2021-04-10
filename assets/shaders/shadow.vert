#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 0) out vec4 v_Pos;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

void main() {
    vec4 p = ViewProj * Model * vec4(Vertex_Position, 1.0);
    gl_Position = p;
    v_Pos = p;
}
