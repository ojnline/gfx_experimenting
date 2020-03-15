#version 450

layout(location = 0) in vec3 position;
layout(location = 0) flat out vec3 frag_norm;

layout(push_constant) uniform Transform {
    mat4 view;
} PushConstants;

void main() {
    frag_norm = normalize(position);
    gl_Position = PushConstants.view * vec4(position, 1.);
}