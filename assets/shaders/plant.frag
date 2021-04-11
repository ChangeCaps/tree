#version 450

layout(location = 0) in vec3 v_Normal;
layout(location = 1) in flat int v_Material;
layout(location = 2) in vec3 v_WorldPos;

layout(location = 0) out vec4 o_Target;

void main() {
    /*
	vec4 projected = ViewProj * vec4(v_WorldPos, 1.0);
    //projected.y *= -1.0;
    float v = 16.0;
    vec2 uv = (projected.xy * vec2(1.0, -1.0) + v) / (2.0 * v);

    float dist = length(v_WorldPos - Pos) / 1000.0;
    */

    vec3 color = vec3(1.0) * 0.1;

    if (v_Material == 0) {
        //color = vec3(155.0 / 255.0, 118.0 / 255.0, 83.0 / 255.0);
    } else {
        //color = vec3(0.1, 0.8, 0.2);
    }

    float sun_diffuse = clamp(dot(v_Normal, vec3(1.0, 1.0, 0.0)), 0.01, 1.0);

    //color *= sun_diffuse;

    o_Target = vec4(color, 1.0);
}
