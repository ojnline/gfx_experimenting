#version 450

layout(location = 0) flat in vec3 frag_norm;
layout(location = 0) out vec4 color;

void main() {
    float light = dot(-frag_norm, vec3(0., 1.,0.));
    float dist = length(frag_norm);
    color = vec4(vec3(light*dist), 1.);
}