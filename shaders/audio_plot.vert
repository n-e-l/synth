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

struct PixelData {
    vec2[2] minmax;
    ivec2 direction;
    uint sample_index;
    uint padding;
};
layout( std430, binding = 0 ) readonly buffer MinMaxBuffer {
    PixelData[] data;
} minmax_data;

vec2 vertex[6] = vec2[](
    vec2( 0.0,  0.0),
    vec2( 0.0,  1.0),
    vec2( 1.0,  0.0),

    vec2( 1.0,  0.0),
    vec2( 0.0,  1.0),
    vec2( 1.0,  1.0)
);

layout(location = 0) flat out vec2 minmax;
layout(location = 1) flat out uint sample_index;

void main()
{
    float pixelwidth = 2. / float(constants.pixels_x);
    float pixelheight = 2. / float(constants.pixels_y);

    float x = 2. * gl_InstanceIndex / constants.pixels_x - 1.;

    PixelData minmax = minmax_data.data[gl_InstanceIndex];
    float minimum = minmax.minmax[constants.channel][0];
    float maximum = minmax.minmax[constants.channel][1];

    // Discard invalid samples
    if(maximum < minimum) return;

    vec2 vertex_p = vertex[gl_VertexIndex];

    // Shift x position in order to have aliased lines
    float x_offset = 0.;
    if( minmax.direction[constants.channel] == 1 ) {
        // Upward line, move the top vertices right by half a pixel
        // Move the bottom pixel left by half a pixel
        x_offset = (vertex_p.y * 2. - 1.) * pixelwidth / 2.;
    } else if (minmax.direction[constants.channel] == 2) {
        // Downward line, move the top vertices left by half a pixel
        // Move the bottom pixel right by half a pixel
        x_offset = -(vertex_p.y * 2. - 1.) * pixelwidth / 2.;
    }

    float height = max(maximum - minimum, pixelheight * 1.);
    vec2 pos = vec2(x + x_offset, minimum) + vertex_p * vec2(pixelwidth, height);
    pos.y = -pos.y;

    sample_index = minmax.sample_index;
    gl_Position = vec4(pos, 0.0, 1.0);
}
