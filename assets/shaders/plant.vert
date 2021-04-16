#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(location = 2) in vec4 Vertex_Color;
layout(location = 3) in float Plant_Sway;
layout(location = 4) in vec2 Vertex_Uv;
layout(location = 5) in uint Plant_Material;

layout(location = 0) out vec3 v_Normal;
layout(location = 1) out vec3 v_Color;
layout(location = 2) out vec3 v_ModelPos;
layout(location = 3) out vec3 v_WorldPos;
layout(location = 4) out vec4 v_ShadowCoord;
layout(location = 5) out float v_Sway;
layout(location = 6) out vec2 v_Uv; 
layout(location = 7) out uint v_Material;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(set = 0, binding = 1) uniform Sun {
    mat4 SunViewProj;
    vec3 SunPos;
};

layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

layout(set = 2, binding = 0) uniform PlantMaterial_time {
    float Time;
};

void main() {
    vec3 model_position = Vertex_Position;

    vec3 world_position = (Model * vec4(model_position, 1.0)).xyz;
    float sway = Plant_Sway;
    sway = pow(sway, 1.3);
    world_position.xz += sin(Time) * sway * 0.002;

    vec4 normal = Model * vec4(Vertex_Normal, 0.0);
    v_Normal = normalize(normal.xyz);

    v_Color = Vertex_Color.rgb;
	v_WorldPos = world_position;

    gl_Position = ViewProj * vec4(world_position, 1.0);
    v_ShadowCoord = SunViewProj * vec4(world_position, 1.0);
    v_Sway = Plant_Sway;
    v_ModelPos = Vertex_Position;
    v_Uv = Vertex_Uv;
    v_Material = Plant_Material;
}
