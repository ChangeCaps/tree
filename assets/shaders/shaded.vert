#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;

layout(location = 0) out vec3 v_Normal;
layout(location = 1) out vec3 v_WorldPos;
layout(location = 2) out vec4 v_ShadowCoord;

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

void main() {
    vec3 model_position = Vertex_Position;

    vec3 world_position = (Model * vec4(model_position, 1.0)).xyz;

    vec4 normal = Model * vec4(Vertex_Normal, 0.0);
    v_Normal = normalize(normal.xyz);

	v_WorldPos = world_position;

    gl_Position = ViewProj * vec4(world_position, 1.0);
    v_ShadowCoord = SunViewProj * vec4(world_position, 1.0);
}
