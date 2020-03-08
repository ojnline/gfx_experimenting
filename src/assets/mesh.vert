#version 450

layout(location = 0) in vec4 pos;
layout(location = 0) out vec3 norm;

void main() {
    norm = pos.xyz;
    gl_Position = pos * 0.9;
}