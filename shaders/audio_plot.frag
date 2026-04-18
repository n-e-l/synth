#version 450

layout( push_constant ) uniform PushConstants
{
    uint samples_per_second;
    uint total_samples;
    uint pixels_x;
    uint pixels_y;
    float zoom;
    uint offset;
} constants;


layout(location = 0) out vec4 outColor;

layout(location = 0) in vec2 minmax;

void main()
{
    vec3 color = vec3(241. / 255.0, 121. / 255., 25. / 255.);
    color = pow(color, vec3(2.2));
    outColor = vec4(color, 1.0);
}
