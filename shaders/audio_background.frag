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

layout(location = 0) in vec2 uv;

layout(location = 0) out vec4 outColor;

void main()
{
    vec3 color = vec3( 0 );

    float offset = constants.offset / float(constants.total_samples);

    float p = offset + (uv.x - .5) * constants.zoom;

    float units_per_pixel = constants.zoom / constants.pixels_x;

    const int MINOR_GRID_LINES_PER_SECOND = 100;
    const int MAJOR_GRID_LINES_PER_SECOND = 10;

    vec3 minor_grid_color = vec3(.02);
    vec3 major_grid_color = vec3(.08);
    if( fract(p * MINOR_GRID_LINES_PER_SECOND) / MINOR_GRID_LINES_PER_SECOND < units_per_pixel ) {
        color = minor_grid_color;
    }

    const int MINOR_GRID_LINES_Y = 4 * 2; // range is from -1 to 1
    if( fract(abs(uv.y - .5) * MINOR_GRID_LINES_Y ) / MINOR_GRID_LINES_Y < 2. / constants.pixels_y ) {
        color = minor_grid_color;
    }

    if( fract(p * MAJOR_GRID_LINES_PER_SECOND) / MAJOR_GRID_LINES_PER_SECOND < units_per_pixel ) {
        color = major_grid_color;
    }
    if( fract(abs(uv.y - .5)) < 1. / constants.pixels_y ) {
        color = major_grid_color;
    }

    outColor = vec4(color, 1.0);

//    outColor = vec4(uv, 0., 1.);
}
