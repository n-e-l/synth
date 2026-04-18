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

layout( std430, binding = 0 ) readonly buffer MinMaxBuffer {
    vec2[] data;
} minmax_data;

vec2 vertex[6] = vec2[](
    vec2( 0.0,  0.0),
    vec2( 1.0,  0.0),
    vec2( 0.0,  1.0),

    vec2( 1.0,  0.0),
    vec2( 1.0,  1.0),
    vec2( 0.0,  1.0)
);

void main()
{
    float pixelwidth = 2. / float(constants.pixels_x);
    float pixelheight = 2. / float(constants.pixels_y);

    float x = 2. * gl_InstanceIndex / constants.pixels_x - 1.;

    vec2 minmax = minmax_data.data[gl_InstanceIndex];

    // Discard invalid samples
    if(minmax[1] < minmax[0]) return;

    float height = max(minmax[1] - minmax[0], pixelheight * 1.);
    vec2 pos = vec2(x, minmax[0]) + vertex[gl_VertexIndex] * vec2(pixelwidth, height);

    gl_Position = vec4(pos, 0.0, 1.0);
}
