#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 color;


layout(set = 0, binding = 1) uniform sampler colorsampler;
layout(set = 0, binding = 0) uniform texture2D colormap;

void main() {
    vec2 uv_ = vec2(uv.x, uv.y + sin(gl_FragCoord.x*0.05)*0.05);
    vec4 pixel = texture(sampler2D(colormap, colorsampler), uv_);
    
    color = vec4(1.-pixel.rgb, pixel.a);
}