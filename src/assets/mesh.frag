#version 450

layout(location = 0) in vec3 norm;
layout(location = 0) out vec4 color;

void main() {
    float light = dot(norm, vec3(1., 0.,0.));
    //light = sin((light+norm.y)*30)*.5 + .5;
    light = 1.-norm.z;
    color = vec4(vec3(light), 1.);
}