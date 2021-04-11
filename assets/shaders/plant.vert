#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in float Plant_Sway;
layout(location = 3) in int Plant_Material;

layout(location = 0) out vec3 v_Normal;
layout(location = 1) out int v_Material;
layout(location = 2) out vec3 v_WorldPos;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

layout(set = 2, binding = 0) uniform PlantMaterial_time {
    float time;
};

void main() {
    vec3 position = Vertex_Position;

    vec3 world_position = (vec4(position, 1.0)).xyz;
    float sway = Plant_Sway;
    sway = pow(sway, 1.3);
    //world_position.xz += sin(time) * sway * 0.005;

    gl_Position = ViewProj * vec4(Vertex_Position, 1.0);
    vec4 normal = Model * vec4(Vertex_Normal, 0.0);
    v_Normal = normal.xyz;

    v_Material = Plant_Material;
	v_WorldPos = world_position;
}
