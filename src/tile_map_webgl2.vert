#version 300 es

layout(location = 0) in vec3 Vertex_Position;
layout(location = 2) in vec2 Vertex_Uv;

out vec2 v_Uv;

layout(std140) uniform Camera {
    mat4 ViewProj;
};

layout(std140) uniform Transform {  // set = 2, binding = 0
    mat4 Model;
};

layout(std140) uniform TileMapChunk {  // set = 2, binding = 1
    float layer_id;
};

void main() {
    v_Uv = Vertex_Uv;
    gl_Position = ViewProj * Model * vec4(Vertex_Position.xy, layer_id, 1.0);
}