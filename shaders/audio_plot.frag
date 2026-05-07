#version 450

layout( push_constant ) uniform PushConstants
{
    uint samples_per_second;
    uint total_samples;
    uint pixels_x;
    uint pixels_y;
    float zoom;
    uint offset;
    uint current_sample;
    uint channel;
} constants;


layout(location = 0) out vec4 outColor;

layout(location = 0) flat in vec2 minmax;
layout(location = 1) flat in uint sample_index;

void main()
{
    vec3 color = vec3(241. / 255.0, 121. / 255., 25. / 255.);

    if( constants.channel == 1 ) color = vec3(25.0 / 255.0, 145.0 / 255.0, 241.0 / 255.0);

    color = pow(color, vec3(2.2));

    outColor = vec4(color, 0.8);
}
