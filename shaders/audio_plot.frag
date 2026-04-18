#version 450

layout( push_constant ) uniform PushConstants
{
    uint samples;
    uint pixels_x;
    uint pixels_y;
    float zoom;
    uint offset;
} constants;

layout(location = 0) out vec4 outColor;

void main()
{
    outColor = vec4(1.0, 0.0, 0.0, 1.0);
}
