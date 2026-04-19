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
} constants;

layout(location = 0) in vec2 uv;

layout(location = 0) out vec4 outColor;

void main()
{
    vec3 color = vec3( 0 );

    float offset = constants.offset / float(constants.samples_per_second);

    // Normalized pixel coord (like uv)
    float p = offset + (uv.x - .5) * constants.zoom;

    float units_per_pixel = constants.zoom / constants.pixels_x;

    const int MINOR_GRID_LINES_PER_SECOND = 100;
    const int MAJOR_GRID_LINES_PER_SECOND = 10;

    vec3 minor_grid_color = vec3(.02) * min(1., 2. / constants.zoom );
    vec3 major_grid_color = vec3(.08) * min(1., 4. / constants.zoom );
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

    float size = 30.;
    float sample_dist = p - constants.current_sample / float(constants.samples_per_second);
    if( constants.current_sample != 0 && abs(sample_dist) < units_per_pixel * size && sample_dist < 0. ) {
        sample_dist = abs(sample_dist);
        float t = clamp((sample_dist * constants.pixels_x) / size, 0., 1.);
        float alpha = pow(1. - t, 20.) + 0.03 * (1. - t);
        vec3 playhead_color = vec3(0.0, 0.6, 0.45) * .7;
        outColor = vec4(mix(color, playhead_color, alpha), 1.0);
    }
}
