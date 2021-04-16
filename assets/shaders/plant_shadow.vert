#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in float Plant_Sway;
layout(location = 2) in vec2 Vertex_Uv;
layout(location = 3) in uint Plant_Material;

layout(location = 0) out vec4 v_Pos;
layout(location = 1) out vec3 v_ModelPos;
layout(location = 2) out vec3 v_WorldPos;
layout(location = 3) out float v_Sway;
layout(location = 4) out vec2 v_Uv;
layout(location = 5) out uint v_Material;

layout(set = 0, binding = 0) uniform Sun {
    mat4 ViewProj;
    vec3 Pos;
};

layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

layout(set = 2, binding = 0) uniform PlantMaterial_time {
    float Time;
};

void main() {
    vec3 world_pos = (Model * vec4(Vertex_Position, 1.0)).xyz;
    float sway = Plant_Sway;
    sway = pow(sway, 1.3);
    world_pos.xz += sin(Time) * sway * 0.002;

    vec4 p = ViewProj * vec4(world_pos, 1.0);
    gl_Position = p;
    v_Pos = p;
    v_WorldPos = world_pos;
    v_Sway = Plant_Sway;
    v_ModelPos = Vertex_Position;
    v_Material = Plant_Material;
    v_Uv = Vertex_Uv;
}
